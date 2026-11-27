use super::{Scene, Shape};
use rapier3d::prelude::*;

pub(super) struct GXPhysics {
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad: BroadPhaseMultiSap,
    narrow: NarrowPhase,

    pub(super) bodies: RigidBodySet,
    colliders: ColliderSet,

    joints: ImpulseJointSet,
    multibody: MultibodyJointSet,
    ccd: CCDSolver,

    pub(super) handles: Vec<Option<RigidBodyHandle>>,
    accumulator: f32,
    pub(super) player: Option<RigidBodyHandle>,
    player_collider: Option<ColliderHandle>,
}

impl GXPhysics {
    pub(super) fn new(scene: &Scene) -> Self {
        let mut world = Self {
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad: BroadPhaseMultiSap::new(),
            narrow: NarrowPhase::new(),

            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),

            joints: ImpulseJointSet::new(),
            multibody: MultibodyJointSet::new(),
            ccd: CCDSolver::new(),

            handles: Vec::new(),
            accumulator: 0.0,
            player: None,
            player_collider: None,
        };

        let ground = world.bodies.insert(
            RigidBodyBuilder::fixed()
                .translation(vector![0.0, -0.5, 0.0])
                .build(),
        );

        world.colliders.insert_with_parent(
            ColliderBuilder::cuboid(50000.0, 0.5, 50000.0)
                .friction(0.8)
                .build(),
            ground,
            &mut world.bodies,
        );

        for object in &scene.objects {
            if object.player_mode != 0 || object.light_only {
                world.handles.push(None);
                continue;
            }
            let builder = if object.dynamic && object.shape != Shape::Mesh {
                RigidBodyBuilder::dynamic()
            } else {
                RigidBodyBuilder::fixed()
            };

            let body = builder
                .translation(vector![
                    object.position[0],
                    object.position[1],
                    object.position[2]
                ])
                .position(Isometry::from_parts(
                    nalgebra::Translation3::new(
                        object.position[0],
                        object.position[1],
                        object.position[2],
                    ),
                    nalgebra::UnitQuaternion::from_euler_angles(
                        object.rotation[0].to_radians(),
                        object.rotation[1].to_radians(),
                        object.rotation[2].to_radians(),
                    ),
                ))
                .ccd_enabled(true)
                .linear_damping(0.05)
                .angular_damping(0.05)
                .build();

            let handle = world.bodies.insert(body);

            let x = object.scale[0].abs().max(0.05) * 0.5;
            let y = object.scale[1].abs().max(0.05) * 0.5;
            let z = object.scale[2].abs().max(0.05) * 0.5;

            let collider = match object.shape {
                Shape::Mesh => {
                    let mut vertices = Vec::new();
                    let mut triangles = Vec::new();

                    for mesh in &object.meshes {
                        let offset = vertices.len() as u32;

                        for p in &mesh.positions {
                            vertices.push(point![
                                p[0] * object.scale[0].max(0.05),
                                p[1] * object.scale[1].max(0.05),
                                p[2] * object.scale[2].max(0.05)
                            ]);
                        }

                        for tri in mesh.indices.chunks_exact(3) {
                            triangles.push([tri[0] + offset, tri[1] + offset, tri[2] + offset]);
                        }
                    }

                    match ColliderBuilder::trimesh(vertices, triangles) {
                        Ok(collider) => collider,

                        Err(_) => ColliderBuilder::cuboid(x, y, z),
                    }
                }
                Shape::Cube => ColliderBuilder::cuboid(x, y, z),

                Shape::Capsule => {
                    let radius = x.min(z) * 0.4;

                    ColliderBuilder::capsule_y((y - radius).max(0.01), radius)
                }

                Shape::Sphere => ColliderBuilder::ball(x.max(y).max(z)),

                Shape::Cylinder => ColliderBuilder::cylinder(y, x.max(z)),
            };

            world.colliders.insert_with_parent(
                collider.mass(1.0).friction(0.7).restitution(0.15).build(),
                handle,
                &mut world.bodies,
            );

            world.handles.push(Some(handle));
        }

        world
    }

    pub(super) fn update(&mut self, scene: &mut Scene, dt: f32) {
        self.accumulator += dt.min(0.1);

        let gravity = vector![0.0, -scene.environment.gravity, 0.0];

        let mut parameters = IntegrationParameters::default();

        parameters.dt = 1.0 / 60.0;

        while self.accumulator >= parameters.dt {
            self.pipeline.step(
                &gravity,
                &parameters,
                &mut self.islands,
                &mut self.broad,
                &mut self.narrow,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.joints,
                &mut self.multibody,
                &mut self.ccd,
                None,
                &(),
                &(),
            );

            self.accumulator -= parameters.dt;
        }

        for (i, handle) in self.handles.iter().enumerate() {
            if let Some(object) = scene.objects.get_mut(i) {
                let Some(handle) = *handle else {
                    continue;
                };

                if let Some(body) = self.bodies.get(handle) {
                    let p = body.translation();

                    object.position = [p.x, p.y, p.z];

                    let (x, y, z) = body.rotation().euler_angles();

                    object.rotation = [x.to_degrees(), y.to_degrees(), z.to_degrees()];
                }
            }
        }
    }
}
impl GXPhysics {
    pub(super) fn spawn_player(&mut self, position: [f32; 3]) {
        if self.player.is_some() {
            return;
        }

        let body = RigidBodyBuilder::dynamic()
            .translation(vector![position[0], position[1].max(1.0), position[2]])
            .lock_rotations()
            .ccd_enabled(true)
            .linear_damping(0.0)
            .can_sleep(false)
            .build();

        let handle = self.bodies.insert(body);

        let collider = self.colliders.insert_with_parent(
            ColliderBuilder::capsule_y(0.55, 0.35)
                .friction(0.0)
                .restitution(0.0)
                .mass(75.0)
                .build(),
            handle,
            &mut self.bodies,
        );

        self.player = Some(handle);
        self.player_collider = Some(collider);
    }

    fn player_grounded(&self) -> bool {
        let Some(collider) = self.player_collider else {
            return false;
        };

        for pair in self.narrow.contact_pairs_with(collider) {
            if !pair.has_any_active_contact {
                continue;
            }

            for manifold in &pair.manifolds {
                let normal = manifold.data.normal;

                let support = if pair.collider1 == collider {
                    -normal.y
                } else {
                    normal.y
                };

                if support > 0.55 {
                    return true;
                }
            }
        }

        false
    }

    pub(super) fn control_player(
        &mut self,
        direction: glam::Vec3,
        speed: f32,
        jump: bool,
        jump_speed: f32,
    ) {
        let grounded = self.player_grounded();

        let Some(handle) = self.player else {
            return;
        };

        let Some(body) = self.bodies.get_mut(handle) else {
            return;
        };

        let mut velocity = *body.linvel();

        velocity.x = direction.x * speed;
        velocity.z = direction.z * speed;

        if jump && grounded {
            velocity.y = jump_speed;
        }

        body.set_linvel(velocity, true);
    }

    pub(super) fn player_camera(&self) -> Option<glam::Vec3> {
        let body = self.bodies.get(self.player?)?;
        let p = body.translation();

        Some(glam::Vec3::new(p.x, p.y + 0.65, p.z))
    }
}

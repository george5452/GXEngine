use super::*;

impl GXEngine {
    pub(super) fn spawn_player_entity(
        &mut self,
        mode: u8,
    ) {
        if self.playing {
            return;
        }

        if self.scene.objects.len() >= MAX_OBJECTS {
            return;
        }

        if self.scene.objects.iter().any(
            |o| o.player_mode != 0
        ) {
            self.status =
                "Scene already contains a player".into();
            return;
        }

        let mut player = Object::new(Shape::Capsule);

        player.name = if mode == 1 {
            "First Person Player".into()
        } else {
            "Third Person Player".into()
        };

        player.position = [0.0, 1.0, 4.0];
        player.rotation = [0.0, 0.0, 0.0];
        player.scale = [1.75, 1.75, 1.75];

        player.color = [0.25, 0.48, 0.95];
        player.metallic = 0.0;
        player.roughness = 0.65;

        player.dynamic = false;
        player.player_mode = mode;

        self.scene.objects.push(player);

        self.selected = Some(
            self.scene.objects.len() - 1
        );

        self.environment_selected = false;

        self.status = "Player created".into();
    }

    pub(super) fn player_update(
        &mut self,
        ctx: &egui::Context,
    ) {
        if !self.playing || !self.mouse_locked {
            return;
        }

        let Some(index) = self.scene.objects.iter()
            .position(|o| o.player_mode != 0)
        else {
            return;
        };

        let player = &self.scene.objects[index];

        let mode = player.player_mode;
        let walk_speed = player.walk_speed;
        let sprint_speed = player.sprint_speed;
        let jump_speed = player.jump_speed;
        let camera_distance = player.camera_distance;

        let forward = Vec3::new(
            self.yaw.cos(),
            0.0,
            self.yaw.sin(),
        ).normalize();

        let right = forward.cross(Vec3::Y);

        let (movement, speed, jump) =
            ctx.input(|input| {
                let mut movement = Vec3::ZERO;

                if input.key_down(egui::Key::W) {
                    movement += forward;
                }

                if input.key_down(egui::Key::S) {
                    movement -= forward;
                }

                if input.key_down(egui::Key::D) {
                    movement += right;
                }

                if input.key_down(egui::Key::A) {
                    movement -= right;
                }

                let speed = if input.modifiers.shift {
                    sprint_speed
                } else {
                    walk_speed
                };

                (
                    movement.normalize_or_zero(),
                    speed,
                    input.key_pressed(egui::Key::Space),
                )
            });

        let direction = self.forward();

        if let Some(world) = &mut self.physics_world {
            world.control_player(
                movement,
                speed,
                jump,
                jump_speed,
            );

            if let Some(head) = world.player_camera() {
                self.scene.objects[index].position =
                    (head - Vec3::Y * 0.65).to_array();

                if mode == 1 {
                    self.camera = head;
                } else {
                    self.camera = head
                        - direction * camera_distance;
                }
            }
        }
    }

    pub(super) fn player_toolbar(
        &mut self,
        ui: &mut egui::Ui,
    ) {
        ui.add_enabled_ui(!self.playing, |ui| {
            ui.menu_button("Add Player", |ui| {
                if ui.button("First Person").clicked() {
                    self.spawn_player_entity(1);
                    ui.close_menu();
                }

                if ui.button("Third Person").clicked() {
                    self.spawn_player_entity(2);
                    ui.close_menu();
                }
            });
        });
    }
}
impl GXEngine {
    pub(super) fn player_mouse(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
    ) {
        let has_player = self.scene.objects.iter()
            .any(|o| o.player_mode != 0);

        if !self.playing || !has_player {
            self.unlock_mouse(ctx);
            return;
        }

        if ctx.input(|i| {
            i.key_pressed(egui::Key::Escape)
            || !i.focused
        }) {
            self.unlock_mouse(ctx);
            return;
        }

        if response.clicked() && !self.mouse_locked {
            self.mouse_locked = true;

            ctx.send_viewport_cmd(
                egui::ViewportCommand::CursorGrab(
                    egui::CursorGrab::Locked
                )
            );

            ctx.send_viewport_cmd(
                egui::ViewportCommand::CursorVisible(false)
            );

            return;
        }

        if self.mouse_locked {
            let delta = ctx.input(|i| {
                i.pointer.motion().unwrap_or(egui::Vec2::ZERO)
            });

            self.yaw += delta.x * 0.0025;

            self.pitch = (
                self.pitch - delta.y * 0.0025
            ).clamp(-1.5, 1.5);
        }
    }

    pub(super) fn unlock_mouse(
        &mut self,
        ctx: &egui::Context,
    ) {
        if !self.mouse_locked {
            return;
        }

        self.mouse_locked = false;

        ctx.send_viewport_cmd(
            egui::ViewportCommand::CursorGrab(
                egui::CursorGrab::None
            )
        );

        ctx.send_viewport_cmd(
            egui::ViewportCommand::CursorVisible(true)
        );
    }
}
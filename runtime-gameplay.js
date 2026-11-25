
import * as pc from "playcanvas";
import RAPIER from "@dimforge/rapier3d-compat";

import {
    Runtime as BaseRuntime,
    Nodes
} from "./runtime-base.js";

export { Nodes };

const clone = data => JSON.parse(JSON.stringify(data));

const vector = v => [v.x, v.y, v.z];

export class Runtime extends BaseRuntime {

    static async create(canvas, project, editor = false) {

        await RAPIER.init();

        let device;

        try {

            device = await pc.createGraphicsDevice(canvas, {
                deviceTypes: ["webgpu", "webgl2"],
                antialias: true
            });

        } catch {

            device = await pc.createGraphicsDevice(canvas, {
                deviceTypes: ["webgl2"],
                antialias: true
            });

        }

        const app = new pc.Application(canvas, {
            graphicsDevice: device,
            mouse: new pc.Mouse(canvas),
            keyboard: new pc.Keyboard(window)
        });

        const engine = new Runtime(app, canvas, editor);

        await engine.load(project);

        app.start();

        return engine;
    }

    constructor(app, canvas, editor) {

        super(app, canvas, editor);

        this.physics = null;
        this.player = null;
        this.accumulator = 0;
        this.jumpRequested = false;

        this.lookYaw = 0;
        this.lookPitch = 0;

        this.gizmos = {};
        this.gizmoDragging = false;

        this.frame = new pc.CameraFrame(
            app,
            this.camera.camera
        );

        if (editor) {

            this.createGizmos();

        }

        this.canvas.addEventListener("click", () => {

            if (this.running && this.player) {

                this.canvas.requestPointerLock?.();

            }

        });

        document.addEventListener("mousemove", event => {

            if (!this.running || !this.player) return;

            if (document.pointerLockElement !== this.canvas) {
                return;
            }

            this.lookYaw -= event.movementX * 0.0025;

            this.lookPitch -= event.movementY * 0.0025;

            this.lookPitch = Math.max(
                -1.35,
                Math.min(1.35, this.lookPitch)
            );
        });

        window.addEventListener("keydown", event => {

            if (!this.running || !this.player) return;

            if (event.code === "Space") {

                event.preventDefault();

                if (!event.repeat) {
                    this.jumpRequested = true;
                }

            }
        });
    }

    // =========================
    // PROFESSIONAL GIZMOS
    // =========================

    createGizmos() {

        const layer = pc.Gizmo.createLayer(this.app);

        this.gizmos = {

            move: new pc.TranslateGizmo(
                this.camera.camera,
                layer
            ),

            rotate: new pc.RotateGizmo(
                this.camera.camera,
                layer
            ),

            scale: new pc.ScaleGizmo(
                this.camera.camera,
                layer
            )

        };

        for (const gizmo of Object.values(this.gizmos)) {

            gizmo.on("transform:start", () => {

                this.gizmoDragging = true;

            });

            gizmo.on("transform:end", () => {

                const entity = this.entities.get(this.selected);

                if (entity) {

                    this.onTransform(this.selected, {

                        position: vector(
                            entity.getLocalPosition()
                        ),

                        rotation: vector(
                            entity.getLocalEulerAngles()
                        ),

                        scale: vector(
                            entity.getLocalScale()
                        )

                    });

                }

                requestAnimationFrame(() => {

                    this.gizmoDragging = false;

                });

            });
        }

        this.gizmoEntity = null;
        this.gizmoTool = null;
    }

    detachGizmos() {

        for (const gizmo of Object.values(this.gizmos || {})) {

            gizmo.detach();

        }

        this.gizmoEntity = null;
        this.gizmoTool = null;
    }

    updateGizmos() {

        if (!this.editor || this.running) return;

        const entity = this.entities.get(this.selected);

        if (
            this.gizmoEntity === entity &&
            this.gizmoTool === this.tool
        ) return;

        this.detachGizmos();

        const gizmo = this.gizmos[this.tool];

        if (entity && gizmo) {

            gizmo.attach([entity]);

        }

        this.gizmoEntity = entity;
        this.gizmoTool = this.tool;
    }

    // Replace the old mouse-drag editing.

    setupInput() {

        let drag = null;

        const canvas = this.canvas;

        canvas.addEventListener("contextmenu", e => {

            e.preventDefault();

        });

        canvas.addEventListener("pointerdown", e => {

            if (!this.editor || this.running) return;

            drag = {

                button: e.button,

                x: e.clientX,
                y: e.clientY,

                startX: e.clientX,
                startY: e.clientY

            };

        });

        canvas.addEventListener("pointermove", e => {

            if (!drag || this.running || this.gizmoDragging) {
                return;
            }

            const dx = e.clientX - drag.x;

            const dy = e.clientY - drag.y;

            drag.x = e.clientX;
            drag.y = e.clientY;

            if (drag.button === 2) {

                this.yaw -= dx * 0.25;

                this.pitch = Math.max(
                    -85,
                    Math.min(85, this.pitch + dy * 0.25)
                );

                this.updateCamera();

            }

            if (drag.button === 1) {

                this.target.add(

                    this.camera.right.clone().mulScalar(
                        -dx * this.distance * 0.002
                    )

                );

                this.target.add(

                    this.camera.up.clone().mulScalar(
                        dy * this.distance * 0.002
                    )

                );

                this.updateCamera();

            }

        });

        window.addEventListener("pointerup", e => {

            if (!drag) return;

            const moved = Math.hypot(

                e.clientX - drag.startX,

                e.clientY - drag.startY

            );

            if (
                !this.running &&
                !this.gizmoDragging &&
                drag.button === 0 &&
                moved < 4
            ) {

                this.selectAt(e.clientX, e.clientY);

            }

            drag = null;

        });

        window.addEventListener("pointercancel", () => {

            drag = null;

        });

        canvas.addEventListener("wheel", e => {

            if (!this.editor || this.running) return;

            e.preventDefault();

            this.distance = Math.max(

                0.5,

                Math.min(
                    1000,
                    this.distance * Math.exp(e.deltaY * 0.001)
                )

            );

            this.updateCamera();

        }, { passive: false });

        window.addEventListener("keydown", e => {

            if (["INPUT", "TEXTAREA", "SELECT"].includes(
                document.activeElement?.tagName
            )) return;

            this.keys.add(e.code);

            if (this.running && !e.repeat) {

                this.fire("KeyPressed", 1, e.code);

            }

        });

        window.addEventListener("keyup", e => {

            this.keys.delete(e.code);

        });

        window.addEventListener("blur", () => {

            this.keys.clear();

        });

    }

    // =========================
    // CINEMATIC RENDERER
    // =========================

    applyGraphics() {

        const settings = this.project.settings;

        const high = settings.quality !== "performance";

        const cinematic = settings.quality === "cinematic";

        const effects = settings.effects || {};

        const frame = this.frame;

        frame.rendering.toneMapping = pc.TONEMAP_ACES;

        frame.rendering.renderTargetScale = high ? 1 : 0.8;

        frame.rendering.sharpness = 0.15;

        frame.ssao.type = (effects.ssao ?? high)
            ? "combine"
            : "none";

        frame.ssao.intensity = 0.55;

        frame.ssao.radius = 5;

        frame.ssao.samples = cinematic ? 24 : 12;

        frame.ssao.scale = cinematic ? 1 : 0.5;

        frame.bloom.enabled = effects.bloom ?? high;

        frame.bloom.intensity = 0.015;

        frame.taa.enabled = effects.taa ?? cinematic;

        frame.grading.enabled = true;

        frame.grading.saturation = 1.04;

        frame.grading.contrast = 1.05;

        frame.vignette.enabled = effects.vignette ?? cinematic;

        frame.vignette.intensity = 0.12;

        frame.dof.enabled = effects.dof ?? false;

        frame.dof.focusDistance = 12;

        frame.dof.focusRange = 8;

        if (frame.volumetricFog) {

            frame.volumetricFog.enabled =
                effects.fog ?? false;

        }

        frame.update();

    }

    async load(project) {

        this.detachGizmos();

        this.destroyPhysics();

        this.sourceProject = clone(project);

        const normalized = clone(project);

        normalized.assets ||= [];

        for (const entity of normalized.entities) {

            if (entity.type === "player-first") {

                entity.playerMode = "first";

                entity.type = "capsule";

            }

            if (entity.type === "player-third") {

                entity.playerMode = "third";

                entity.type = "capsule";

            }

        }

        await super.load(normalized);

        this.applyGraphics();

    }

    // =========================
    // PHYSICS WORLD
    // =========================

    destroyPhysics() {

        if (this.physics) {

            this.physics.free();

            this.physics = null;

        }

        this.player = null;
        this.accumulator = 0;

    }

    createPhysics() {

        this.destroyPhysics();

        this.physics = new RAPIER.World({

            x: 0,
            y: -24,
            z: 0

        });

        for (const data of this.project.entities) {

            if (data.playerMode) continue;

            if (data.collision === false) continue;

            if (["camera", "empty"].includes(data.type)) {
                continue;
            }

            const entity = this.entities.get(data.id);

            if (!entity) continue;

            this.createStaticCollision(entity);

        }

        this.physics.timestep = 1 / 60;

        this.physics.step();

    }

    // Actual triangle-mesh collisions for static geometry.

    createStaticCollision(entity) {

        const renders = entity.findComponents("render");

        for (const render of renders) {

            for (const instance of render.meshInstances) {

                const positions = [];
                const indices = [];

                const mesh = instance.mesh;

                mesh.getPositions(positions);

                const count = mesh.getIndices(indices);

                if (!positions.length) continue;

                const primitive = mesh.primitive[0];

                if (!primitive || primitive.type !== pc.PRIMITIVE_TRIANGLES) {
                    continue;
                }

                let triangles;

                if (count && primitive.indexed) {

                    triangles = indices.slice(
                        primitive.base,
                        primitive.base + primitive.count
                    );

                } else {

                    triangles = Array.from(
                        { length: primitive.count },
                        (_, i) => i + primitive.base
                    );

                }

                if (triangles.length < 3) continue;

                const matrix = instance.node.getWorldTransform();

                const point = new pc.Vec3();

                for (let i = 0; i < positions.length; i += 3) {

                    point.set(

                        positions[i],

                        positions[i + 1],

                        positions[i + 2]

                    );

                    matrix.transformPoint(point, point);

                    positions[i] = point.x;

                    positions[i + 1] = point.y;

                    positions[i + 2] = point.z;

                }

                try {

                    const desc = RAPIER.ColliderDesc.trimesh(

                        new Float32Array(positions),

                        new Uint32Array(triangles)

                    );

                    desc.setFriction(0.8);

                    this.physics.createCollider(desc);

                } catch (error) {

                    this.onMessage(
                        "Collision skipped: " + entity.name
                    );

                }
            }
        }

    }

    // =========================
    // CHARACTER CONTROLLER
    // =========================

    createPlayer(data) {

        const entity = this.entities.get(data.id);

        const position = entity.getPosition();

        const body = this.physics.createRigidBody(

            RAPIER.RigidBodyDesc
                .kinematicPositionBased()
                .setTranslation(
                    position.x,
                    position.y,
                    position.z
                )

        );

        const collider = this.physics.createCollider(

            RAPIER.ColliderDesc.capsule(0.5, 0.35),

            body

        );

        const controller = this.physics
            .createCharacterController(0.02);

        controller.enableAutostep(0.35, 0.2, false);

        controller.enableSnapToGround(0.2);

        controller.setMaxSlopeClimbAngle(
            Math.PI / 4
        );

        controller.setMinSlopeSlideAngle(
            Math.PI / 3
        );

        this.player = {

            entity,
            body,
            collider,
            controller,

            mode: data.playerMode,

            verticalVelocity: 0,

            grounded: false,

            speed: data.walkSpeed ?? 5,

            sprint: data.sprintSpeed ?? 9,

            jump: data.jumpSpeed ?? 8,

            cameraDistance: data.cameraDistance ?? 6

        };

        this.lookYaw = entity.getEulerAngles().y * Math.PI / 180;

        this.lookPitch = 0;

        if (this.player.mode === "first") {

            for (const render of entity.findComponents("render")) {

                render.enabled = false;

            }

        }

        this.physics.step();

        this.updatePlayerCamera();

    }

    physicsStep(dt) {

        const player = this.player;

        if (!player) {

            this.physics.step();

            return;

        }

        const forward =

            Number(this.keys.has("KeyW")) -

            Number(this.keys.has("KeyS"));

        const strafe =

            Number(this.keys.has("KeyD")) -

            Number(this.keys.has("KeyA"));

        const length = Math.hypot(forward, strafe) || 1;

        const speed = this.keys.has("ShiftLeft")

            ? player.sprint

            : player.speed;

        const yaw = this.lookYaw;

        const x = (

            Math.cos(yaw) * strafe -

            Math.sin(yaw) * forward

        ) / length;

        const z = (

            -Math.cos(yaw) * forward -

            Math.sin(yaw) * strafe

        ) / length;

        if (player.grounded && player.verticalVelocity < 0) {

            player.verticalVelocity = -0.5;

        }

        if (this.jumpRequested && player.grounded) {

            player.verticalVelocity = player.jump;

        }

        this.jumpRequested = false;

        player.verticalVelocity -= 24 * dt;

        player.controller.computeColliderMovement(

            player.collider,

            {

                x: x * speed * dt,

                y: player.verticalVelocity * dt,

                z: z * speed * dt

            }

        );

        const movement = player.controller.computedMovement();

        const position = player.body.translation();

        player.body.setNextKinematicTranslation({

            x: position.x + movement.x,

            y: position.y + movement.y,

            z: position.z + movement.z

        });

        player.grounded = player.controller.computedGrounded();

        this.physics.step();

        const next = player.body.translation();

        player.entity.setPosition(
            next.x,
            next.y,
            next.z
        );

        if (forward || strafe) {

            player.entity.setEulerAngles(

                0,

                Math.atan2(-x, -z) * 180 / Math.PI,

                0

            );

        }

        // Basic respawn if the player falls off the map.

        if (next.y < -100) {

            const data = this.project.entities.find(
                e => e.playerMode
            );

            const [x, y, z] = data.position;

            player.body.setTranslation({ x, y, z }, true);

            player.entity.setPosition(x, y, z);

            player.verticalVelocity = 0;

        }

    }

    updatePlayerCamera() {

        if (!this.player) return;

        const player = this.player;

        const p = player.entity.getPosition();

        const yaw = this.lookYaw;

        const pitch = this.lookPitch;

        const direction = new pc.Vec3(

            -Math.sin(yaw) * Math.cos(pitch),

            Math.sin(pitch),

            -Math.cos(yaw) * Math.cos(pitch)

        );

        const head = new pc.Vec3(

            p.x,

            p.y + 0.65,

            p.z

        );

        if (player.mode === "first") {

            this.camera.setPosition(head);

            this.camera.lookAt(

                head.clone().add(direction)

            );

            return;

        }

        // Third-person orbit camera with obstacle checks.

        const behind = direction.clone().mulScalar(-1);

        const ray = new RAPIER.Ray(

            head,

            behind

        );

        const hit = this.physics.castRay(

            ray,

            player.cameraDistance,

            true,

            undefined,

            undefined,

            player.collider,

            player.body

        );

        let distance = player.cameraDistance;

        if (hit) {

            distance = Math.max(

                0.2,

                (hit.timeOfImpact ?? hit.toi ?? distance) - 0.2

            );

        }

        const position = head.clone().add(

            behind.mulScalar(distance)

        );

        this.camera.setPosition(position);

        this.camera.lookAt(head);

    }

    // =========================
    // GAME LIFECYCLE
    // =========================

    play() {

        this.detachGizmos();

        this.createPhysics();

        this.lookYaw = 0;

        this.lookPitch = 0;

        super.play();

        const player = this.project.entities.find(

            entity => entity.playerMode

        );

        if (player) {

            this.createPlayer(player);

            this.onMessage(

                "Click viewport to control player"

            );

        }

    }

    async stop() {

        this.running = false;

        if (document.pointerLockElement === this.canvas) {

            document.exitPointerLock();

        }

        this.destroyPhysics();

        await this.load(this.sourceProject);

    }

    update(dt) {

        if (!this.running) {

            this.drawGrid();

            this.updateGizmos();

            return;

        }

        dt = Math.min(dt, 0.1);

        this.fire("Update", dt);

        for (const key of this.keys) {

            this.fire("KeyHeld", dt, key);

        }

        if (!this.running) return;

        this.accumulator += dt;

        const step = 1 / 60;

        while (
            this.physics &&
            this.accumulator >= step
        ) {

            this.physicsStep(step);

            this.accumulator -= step;

        }

        if (this.player) {

            this.updatePlayerCamera();

        } else {

            this.syncGameCamera();

        }

    }

}

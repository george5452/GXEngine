
import * as pc from "playcanvas";
import RAPIER from "@dimforge/rapier3d-compat";

import {
    Runtime as GameplayRuntime,
    Nodes as OriginalNodes
} from "./runtime-gameplay.js";

export const Nodes = {
    ...OriginalNodes,

    TriggerEnter: {
        category: "Event",
        defaults: {}
    },

    ApplyImpulse: {
        category: "Physics",
        defaults: {
            x: 0,
            y: 8,
            z: 0
        }
    },

    SetVelocity: {
        category: "Physics",
        defaults: {
            x: 0,
            y: 0,
            z: 0
        }
    },

    DestroySelf: {
        category: "Action",
        defaults: {}
    },

    SetCheckpoint: {
        category: "Action",
        defaults: {}
    },

    FinishLevel: {
        category: "Action",
        defaults: {
            message: "Level Complete!"
        }
    }
};

export const PhysicsDefaults = {

    body: "static",
    shape: "auto",

    mass: 1,
    friction: 0.7,
    restitution: 0.1,

    gravity: 1,
    linearDamping: 0.05,
    angularDamping: 0.05,

    ccd: true

};

export class Runtime extends GameplayRuntime {

    static async create(canvas, project, editor = false) {

        await RAPIER.init();

        const device = await pc.createGraphicsDevice(canvas, {
            deviceTypes: ["webgpu", "webgl2"],
            antialias: true
        });

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

        this.bodies = new Map();
        this.triggers = new Map();
        this.destroyed = new Set();

        this.checkpoint = null;
        this.levelFinished = false;

    }

    // PHYSICS SHAPES

    makeCollider(data) {

        const settings = {
            ...PhysicsDefaults,
            ...data.physics
        };

        const [sx, sy, sz] = data.scale.map(
            value => Math.max(0.01, Math.abs(value))
        );

        let shape = settings.shape;

        if (shape === "auto") {

            shape =
                data.type === "sphere" ? "sphere" :
                data.type === "capsule" ? "capsule" :
                data.type === "cylinder" ? "cylinder" :
                "box";

        }

        if (shape === "sphere") {

            return RAPIER.ColliderDesc.ball(
                Math.max(sx, sy, sz) * 0.5
            );

        }

        if (shape === "capsule") {

            const radius = Math.min(sx, sz) * 0.5;

            return RAPIER.ColliderDesc.capsule(
                Math.max(0.01, sy * 0.5 - radius),
                radius
            );

        }

        if (shape === "cylinder") {

            return RAPIER.ColliderDesc.cylinder(
                sy * 0.5,
                Math.max(sx, sz) * 0.5
            );

        }

        return RAPIER.ColliderDesc.cuboid(
            sx * 0.5,
            data.type === "plane" ? 0.025 : sy * 0.5,
            sz * 0.5
        );

    }

    createPhysics() {

        this.destroyPhysics();

        this.bodies = new Map();
        this.triggers = new Map();
        this.destroyed = new Set();

        this.checkpoint = null;
        this.levelFinished = false;

        this.physics = new RAPIER.World({
            x: 0,
            y: -24,
            z: 0
        });

        this.physics.timestep = 1 / 60;

        for (const data of this.project.entities) {

            if (data.playerMode) continue;

            if (data.collision === false) continue;

            if (["empty", "camera"].includes(data.type)) {
                continue;
            }

            const entity = this.entities.get(data.id);

            if (!entity) continue;

            const settings = {
                ...PhysicsDefaults,
                ...data.physics
            };

            if (settings.body === "none") continue;

            // Static meshes use accurate triangle collision.

            if (
                settings.body === "static" &&
                (
                    settings.shape === "mesh" ||
                    settings.shape === "auto"
                )
            ) {

                this.createStaticCollision(entity);

                continue;
            }

            let description;

            switch (settings.body) {

                case "dynamic":
                    description = RAPIER.RigidBodyDesc.dynamic();
                    break;

                case "kinematic":
                    description =
                        RAPIER.RigidBodyDesc.kinematicPositionBased();
                    break;

                default:
                    description = RAPIER.RigidBodyDesc.fixed();

            }

            const position = entity.getPosition();
            const rotation = entity.getRotation();

            description.setTranslation(
                position.x,
                position.y,
                position.z
            );

            description.setRotation({
                x: rotation.x,
                y: rotation.y,
                z: rotation.z,
                w: rotation.w
            });

            if (settings.body === "dynamic") {

                description.setGravityScale(
                    Number(settings.gravity)
                );

                description.setLinearDamping(
                    Number(settings.linearDamping)
                );

                description.setAngularDamping(
                    Number(settings.angularDamping)
                );

                description.setCcdEnabled(
                    Boolean(settings.ccd)
                );

            }

            const body = this.physics.createRigidBody(
                description
            );

            const colliderDescription = this.makeCollider({
                ...data,
                physics: settings
            });

            colliderDescription.setFriction(
                Number(settings.friction)
            );

            colliderDescription.setRestitution(
                Number(settings.restitution)
            );

            if (settings.body === "dynamic") {

                colliderDescription.setMass(
                    Math.max(0.01, Number(settings.mass))
                );

            }

            if (settings.body === "trigger") {

                colliderDescription.setSensor(true);

                colliderDescription.setActiveCollisionTypes(
                    RAPIER.ActiveCollisionTypes.DEFAULT |
                    RAPIER.ActiveCollisionTypes.KINEMATIC_FIXED
                );

            }

            const collider = this.physics.createCollider(
                colliderDescription,
                body
            );

            this.bodies.set(data.id, {

                body,
                collider,
                entity,
                settings

            });

            if (settings.body === "trigger") {

                this.triggers.set(data.id, {

                    collider,
                    inside: false,
                    data

                });

            }

        }

        this.physics.step();

    }

    createPlayer(data) {

        super.createPlayer(data);

        this.player.controller.setApplyImpulsesToDynamicBodies(
            true
        );

        this.player.controller.setCharacterMass(75);

        this.checkpoint = {
            ...this.player.body.translation()
        };

        // Sensors must not block the character.

        const controller = this.player.controller;

        const compute = controller.computeColliderMovement.bind(
            controller
        );

        controller.computeColliderMovement = (
            collider,
            movement,
            flags,
            groups,
            predicate
        ) => {

            return compute(

                collider,

                movement,

                (flags || 0) |
                    RAPIER.QueryFilterFlags.EXCLUDE_SENSORS,

                groups,

                predicate

            );

        };

    }

    physicsStep(dt) {

        if (!this.physics) return;

        // Kinematic objects follow their Blueprint transforms.

        for (const [id, object] of this.bodies) {

            if (this.destroyed.has(id)) continue;

            if (object.settings.body === "kinematic") {

                const p = object.entity.getPosition();

                const q = object.entity.getRotation();

                object.body.setNextKinematicTranslation({
                    x: p.x,
                    y: p.y,
                    z: p.z
                });

                object.body.setNextKinematicRotation({
                    x: q.x,
                    y: q.y,
                    z: q.z,
                    w: q.w
                });

            }

        }

        // Check respawn before the old player routine runs.

        if (
            this.player &&
            this.player.body.translation().y < -50 &&
            this.checkpoint
        ) {

            this.player.body.setTranslation(
                this.checkpoint,
                true
            );

            this.player.body.setNextKinematicTranslation(
                this.checkpoint
            );

            this.player.verticalVelocity = 0;

            this.player.entity.setPosition(

                this.checkpoint.x,
                this.checkpoint.y,
                this.checkpoint.z

            );

        }

        super.physicsStep(dt);

        // Synchronize dynamic bodies with graphics.

        for (const [id, object] of this.bodies) {

            if (this.destroyed.has(id)) continue;

            if (object.settings.body !== "dynamic") continue;

            const p = object.body.translation();
            const q = object.body.rotation();

            object.entity.setPosition(
                p.x, p.y, p.z
            );

            object.entity.setRotation(

                new pc.Quat(q.x, q.y, q.z, q.w)

            );

        }

        this.checkTriggers();

    }

    checkTriggers() {

        if (!this.player) return;

        for (const [id, trigger] of this.triggers) {

            if (this.destroyed.has(id)) continue;

            const inside = this.physics.intersectionPair(

                trigger.collider,

                this.player.collider

            );

            if (inside && !trigger.inside) {

                this.fireEntityEvent(

                    id,
                    "TriggerEnter"

                );

            }

            trigger.inside = inside;

        }

    }

    fireEntityEvent(id, event) {

        const data = this.project.entities.find(
            item => item.id === id
        );

        const entity = this.entities.get(id);

        if (!data || !entity) return;

        if (this.destroyed.has(id)) return;

        const graph = data.blueprint;

        if (!graph) return;

        for (const node of graph.nodes) {

            if (node.type === event) {

                this.execute(

                    graph,
                    node.id,
                    entity,
                    1

                );

            }

        }

    }

    // EXTENDED BLUEPRINT EXECUTION

    execute(graph, start, entity, dt) {

        const entityID = [...this.entities.entries()]
            .find(([, value]) => value === entity)?.[0];

        if (this.destroyed?.has(entityID)) return;

        const map = new Map(

            graph.nodes.map(node => [node.id, node])

        );

        const queue = [start];

        let steps = 0;

        while (queue.length && steps++ < 1024) {

            if (this.destroyed.has(entityID)) break;

            const id = queue.shift();

            const node = map.get(id);

            if (!node) continue;

            const p = node.params || {};

            const object = this.bodies?.get(entityID);

            const v = {

                x: Number(p.x) || 0,

                y: Number(p.y) || 0,

                z: Number(p.z) || 0

            };

            let output = "next";
            let repeat = 1;

            switch (node.type) {

                case "ApplyImpulse":

                    if (object?.settings.body === "dynamic") {

                        object.body.applyImpulse(v, true);

                    }

                    break;

                case "SetVelocity":

                    if (object?.settings.body === "dynamic") {

                        object.body.setLinvel(v, true);

                    }

                    break;

                case "DestroySelf":

                    if (object) {

                        this.physics.removeRigidBody(object.body);

                        this.bodies.delete(entityID);

                    }

                    this.triggers.delete(entityID);

                    this.destroyed.add(entityID);

                    entity.enabled = false;

                    break;

                case "SetCheckpoint":

                    if (this.player) {

                        this.checkpoint = {
                            ...this.player.body.translation()
                        };

                        this.onMessage("Checkpoint reached!");

                    }

                    break;

                case "FinishLevel":

                    this.levelFinished = true;

                    this.onMessage(
                        p.message || "Level Complete!"
                    );

                    this.running = false;

                    if (
                        document.pointerLockElement === this.canvas
                    ) {

                        document.exitPointerLock();

                    }

                    break;

                case "Branch":

                    output =

                        (this.variables[p.name] || 0) >
                        Number(p.greaterThan)

                        ? "true" : "false";

                    break;

                case "Repeat":

                    repeat = Math.max(

                        0,

                        Math.min(
                            100,
                            Math.floor(Number(p.count))
                        )

                    );

                    break;

                default:

                    // Reuse existing nodes without following
                    // their wires twice.

                    super.execute({

                        nodes: [node],
                        links: []

                    }, node.id, entity, dt);

                    break;

            }

            if (
                this.destroyed.has(entityID) ||
                this.levelFinished
            ) break;

            for (let i = 0; i < repeat; i++) {

                for (const link of graph.links) {

                    if (
                        link.from === id &&
                        link.output === output
                    ) {

                        queue.push(link.to);

                    }

                }

            }

        }

        if (queue.length && steps >= 1024) {

            this.running = false;

            this.onMessage(
                "Blueprint execution limit exceeded."
            );

        }

    }

}

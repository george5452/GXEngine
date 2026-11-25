
import * as pc from "playcanvas";

export const Nodes = {

    Start: {
        category: "Event",
        defaults: {}
    },

    Update: {
        category: "Event",
        defaults: {}
    },

    KeyPressed: {
        category: "Event",
        defaults: { key: "Space" }
    },

    KeyHeld: {
        category: "Event",
        defaults: { key: "KeyW" }
    },

    Move: {
        category: "Action",
        defaults: { x: 0, y: 0, z: -5 }
    },

    Rotate: {
        category: "Action",
        defaults: { x: 0, y: 45, z: 0 }
    },

    SetPosition: {
        category: "Action",
        defaults: { x: 0, y: 1, z: 0 }
    },

    SetScale: {
        category: "Action",
        defaults: { x: 1, y: 1, z: 1 }
    },

    SetColor: {
        category: "Action",
        defaults: { color: "#ff8844" }
    },

    SetVisible: {
        category: "Action",
        defaults: { visible: "true" }
    },

    Print: {
        category: "Action",
        defaults: { text: "Hello!" }
    },

    SetVariable: {
        category: "Variable",
        defaults: { name: "score", value: 0 }
    },

    AddVariable: {
        category: "Variable",
        defaults: { name: "score", value: 1 }
    },

    MultiplyVariable: {
        category: "Variable",
        defaults: { name: "score", value: 2 }
    },

    ShowVariable: {
        category: "Variable",
        defaults: { name: "score" }
    },

    Random: {
        category: "Variable",
        defaults: { name: "result", min: 0, max: 100 }
    },

    Branch: {
        category: "Logic",
        defaults: { name: "score", greaterThan: 10 }
    },

    Repeat: {
        category: "Logic",
        defaults: { count: 5 }
    }
};

const clone = data => JSON.parse(JSON.stringify(data));

const col = hex => new pc.Color().fromString(hex);

export class Runtime {

    static async create(canvas, project, editor = false) {

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

        this.app = app;
        this.canvas = canvas;
        this.editor = editor;

        this.running = false;
        this.selected = null;
        this.tool = "select";
        this.gridVisible = true;

        this.entities = new Map();
        this.resources = new Map();
        this.materials = [];

        this.variables = {};
        this.keys = new Set();

        this.onMessage = console.log;
        this.onSelect = () => {};
        this.onTransform = () => {};

        this.world = new pc.Entity("World");
        app.root.addChild(this.world);

        this.camera = new pc.Entity("Viewport Camera");

        this.camera.addComponent("camera", {
            clearColor: col("#8c9cad"),
            fov: 60,
            nearClip: 0.05,
            farClip: 5000
        });

        app.root.addChild(this.camera);

        this.sun = new pc.Entity("Sun");

        this.sun.addComponent("light", {
            type: "directional",
            color: new pc.Color(1, 0.96, 0.9),
            intensity: 2,
            castShadows: true,
            shadowResolution: 2048,
            shadowDistance: 100,
            shadowBias: 0.1,
            normalOffsetBias: 0.04
        });

        this.sun.setEulerAngles(45, 35, 0);
        app.root.addChild(this.sun);

        app.scene.ambientLight = new pc.Color(
            0.36, 0.39, 0.44
        );

        app.scene.toneMapping = pc.TONEMAP_ACES;

        this.target = new pc.Vec3(0, 1, 0);
        this.yaw = 45;
        this.pitch = 25;
        this.distance = 15;

        this.updateCamera();

        app.setCanvasFillMode(pc.FILLMODE_NONE);
        app.setCanvasResolution(pc.RESOLUTION_AUTO);

        this.resize = () => {

            const parent = canvas.parentElement;

            const width = Math.floor(parent.clientWidth);
            const height = Math.floor(parent.clientHeight);

            if (!width || !height) return;

            app.resizeCanvas(width, height);
        };

        this.observer = new ResizeObserver(this.resize);
        this.observer.observe(canvas.parentElement);

        this.resize();

        this.setupInput();

        app.on("update", dt => this.update(dt));
    }

    updateCamera() {

        const y = this.yaw * Math.PI / 180;
        const p = this.pitch * Math.PI / 180;

        this.camera.setPosition(
            this.target.x + Math.sin(y) * Math.cos(p) * this.distance,
            this.target.y + Math.sin(p) * this.distance,
            this.target.z + Math.cos(y) * Math.cos(p) * this.distance
        );

        this.camera.lookAt(this.target);
    }

    async loadAsset(data) {

        if (this.resources.has(data.id)) {
            return this.resources.get(data.id);
        }

        const response = await fetch(data.data);
        const blob = await response.blob();

        const url = URL.createObjectURL(blob);

        try {

            const asset = new pc.Asset(
                data.name,
                data.type === "model" ? "container" : "texture",
                {
                    url,
                    filename: data.name
                }
            );

            this.app.assets.add(asset);

            await new Promise((resolve, reject) => {

                asset.ready(resolve);
                asset.once("error", reject);

                this.app.assets.load(asset);

            });

            this.resources.set(data.id, asset);

            return asset;

        } finally {

            URL.revokeObjectURL(url);

        }
    }

    async load(project) {

        this.running = false;

        this.project = clone(project);

        for (const child of [...this.world.children]) {
            child.destroy();
        }

        for (const material of this.materials) {
            material.destroy();
        }

        this.materials = [];
        this.entities.clear();

        for (const data of this.project.entities) {

            const entity = new pc.Entity(data.name);

            entity.setLocalPosition(...data.position);
            entity.setLocalEulerAngles(...data.rotation);
            entity.setLocalScale(...data.scale);

            if (data.type === "model") {

                const assetData = this.project.assets.find(
                    asset => asset.id === data.asset
                );

                if (assetData) {

                    const asset = await this.loadAsset(assetData);

                    entity.addChild(
                        asset.resource.instantiateRenderEntity()
                    );

                }

            } else if (
                data.type !== "camera" &&
                data.type !== "empty"
            ) {

                entity.addComponent("render", {
                    type: data.type,
                    castShadows: true,
                    receiveShadows: true
                });

                const material = new pc.StandardMaterial();

                material.diffuse = col(data.color || "#ffffff");
                material.useMetalness = true;
                material.metalness = data.metalness ?? 0;
                material.gloss = 1 - (data.roughness ?? 0.5);

                if (data.texture) {

                    const textureData = this.project.assets.find(
                        asset => asset.id === data.texture
                    );

                    if (textureData) {

                        const texture = await this.loadAsset(textureData);
                        material.diffuseMap = texture.resource;

                    }
                }

                material.update();

                entity.render.material = material;

                this.materials.push(material);
            }

            this.world.addChild(entity);

            this.entities.set(data.id, entity);
        }

        const settings = project.settings;

        this.camera.camera.clearColor = col(settings.sky);

        this.sun.light.intensity = settings.sun;

        this.app.scene.exposure = settings.exposure;

        const quality = settings.quality;

        this.sun.light.shadowResolution =
            quality === "cinematic" ? 4096 :
            quality === "high" ? 2048 : 1024;

        this.app.graphicsDevice.maxPixelRatio =
            quality === "performance"
                ? 1
                : Math.min(window.devicePixelRatio || 1, 2);

        this.resize();
        this.updateCamera();
    }

    selectAt(x, y) {

        const rect = this.canvas.getBoundingClientRect();

        const sx = x - rect.left;
        const sy = y - rect.top;

        const start = this.camera.camera.screenToWorld(
            sx, sy, this.camera.camera.nearClip
        );

        const end = this.camera.camera.screenToWorld(
            sx, sy, this.camera.camera.farClip
        );

        const ray = new pc.Ray(
            start,
            end.clone().sub(start).normalize()
        );

        let closest = Infinity;
        let selected = null;

        for (const [id, entity] of this.entities) {

            const components = entity.findComponents("render");

            for (const render of components) {

                for (const mesh of render.meshInstances) {

                    const hit = new pc.Vec3();

                    if (!mesh.aabb.intersectsRay(ray, hit)) {
                        continue;
                    }

                    const distance = hit.distance(start);

                    if (distance < closest) {

                        closest = distance;
                        selected = id;

                    }
                }
            }
        }

        this.selected = selected;

        this.onSelect(selected);
    }

    setupInput() {

        let drag = null;

        this.canvas.addEventListener("contextmenu",
            e => e.preventDefault()
        );

        this.canvas.addEventListener("pointerdown", e => {

            if (!this.editor || this.running) return;

            drag = {
                button: e.button,
                x: e.clientX,
                y: e.clientY,
                startX: e.clientX,
                startY: e.clientY
            };

            this.canvas.setPointerCapture(e.pointerId);
        });

        this.canvas.addEventListener("pointermove", e => {

            if (!drag || this.running) return;

            const dx = e.clientX - drag.x;
            const dy = e.clientY - drag.y;

            drag.x = e.clientX;
            drag.y = e.clientY;

            if (drag.button === 2) {

                this.yaw -= dx * .3;

                this.pitch = Math.max(
                    -85,
                    Math.min(85, this.pitch + dy * .3)
                );

                this.updateCamera();

            } else if (drag.button === 1) {

                this.target.add(
                    this.camera.right.clone()
                        .mulScalar(-dx * this.distance * .002)
                );

                this.target.add(
                    this.camera.up.clone()
                        .mulScalar(dy * this.distance * .002)
                );

                this.updateCamera();

            } else if (
                drag.button === 0 &&
                this.selected &&
                this.tool !== "select"
            ) {

                const entity = this.entities.get(this.selected);

                if (!entity) return;

                if (this.tool === "move") {

                    const right = this.camera.right.clone();
                    right.y = 0;
                    right.normalize();

                    entity.translate(
                        right.mulScalar(dx * this.distance * .002)
                    );

                    entity.translate(
                        0,
                        -dy * this.distance * .002,
                        0
                    );
                }

                if (this.tool === "rotate") {

                    entity.rotateLocal(
                        dy * .5,
                        dx * .5,
                        0
                    );
                }

                if (this.tool === "scale") {

                    const factor = Math.exp(dx * .008);

                    entity.setLocalScale(
                        entity.getLocalScale().clone()
                            .mulScalar(factor)
                    );
                }
            }
        });

        this.canvas.addEventListener("pointerup", e => {

            if (!drag) return;

            const distance = Math.hypot(
                e.clientX - drag.startX,
                e.clientY - drag.startY
            );

            if (drag.button === 0) {

                if (distance < 4) {

                    this.selectAt(e.clientX, e.clientY);

                } else if (this.selected && this.tool !== "select") {

                    const entity = this.entities.get(this.selected);

                    if (entity) {

                        this.onTransform(this.selected, {
                            position: entity.getLocalPosition().toArray(),
                            rotation: entity.getLocalEulerAngles().toArray(),
                            scale: entity.getLocalScale().toArray()
                        });

                    }
                }
            }

            drag = null;
        });

        this.canvas.addEventListener("pointercancel", () => {
            drag = null;
        });

        this.canvas.addEventListener("wheel", e => {

            if (!this.editor || this.running) return;

            e.preventDefault();

            this.distance = Math.max(
                .5,
                Math.min(
                    1000,
                    this.distance * Math.exp(e.deltaY * .001)
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

    drawGrid() {

        if (!this.editor || this.running || !this.gridVisible) {
            return;
        }

        const color = new pc.Color(.45, .45, .48);

        for (let i = -20; i <= 20; i++) {

            this.app.drawLine(
                new pc.Vec3(i, .005, -20),
                new pc.Vec3(i, .005, 20),
                color
            );

            this.app.drawLine(
                new pc.Vec3(-20, .005, i),
                new pc.Vec3(20, .005, i),
                color
            );
        }
    }

    play() {

        this.variables = {};
        this.keys.clear();
        this.running = true;

        this.gameCamera = this.project.entities.find(
            entity => entity.type === "camera"
        );

        this.syncGameCamera();

        this.fire("Start", 1);
    }

    syncGameCamera() {

        if (!this.gameCamera) return;

        const entity = this.entities.get(this.gameCamera.id);

        if (entity) {

            this.camera.setPosition(entity.getPosition());
            this.camera.setRotation(entity.getRotation());

        }
    }

    async stop() {

        this.running = false;

        await this.load(this.project);
    }

    update(dt) {

        this.drawGrid();

        if (!this.running) return;

        dt = Math.min(dt, .1);

        this.fire("Update", dt);

        for (const key of this.keys) {

            this.fire("KeyHeld", dt, key);

        }

        this.syncGameCamera();
    }

    fire(event, dt, key = null) {

        for (const data of this.project.entities) {

            const entity = this.entities.get(data.id);

            const graph = data.blueprint;

            if (!entity || !graph) continue;

            for (const node of graph.nodes) {

                if (node.type !== event) continue;

                if (
                    ["KeyPressed", "KeyHeld"].includes(event) &&
                    node.params.key !== key
                ) continue;

                this.execute(graph, node.id, entity, dt);
            }
        }
    }

    execute(graph, start, entity, dt) {

        const queue = [start];

        const map = new Map(
            graph.nodes.map(node => [node.id, node])
        );

        let steps = 0;

        while (queue.length && steps++ < 1024) {

            const id = queue.shift();
            const node = map.get(id);

            if (!node) continue;

            const p = node.params || {};

            let output = "next";
            let repeat = 1;

            const vector = () => [
                Number(p.x) || 0,
                Number(p.y) || 0,
                Number(p.z) || 0
            ];

            switch (node.type) {

                case "Move":

                    entity.translateLocal(
                        ...vector().map(v => v * dt)
                    );

                    break;

                case "Rotate":

                    entity.rotateLocal(
                        ...vector().map(v => v * dt)
                    );

                    break;

                case "SetPosition":

                    entity.setPosition(...vector());

                    break;

                case "SetScale":

                    entity.setLocalScale(...vector());

                    break;

                case "SetColor":

                    if (entity.render) {

                        entity.render.material.diffuse = col(p.color);

                        entity.render.material.update();

                    }

                    break;

                case "SetVisible":

                    for (const render of entity.findComponents("render")) {

                        render.enabled = String(p.visible) === "true";

                    }

                    break;

                case "Print":

                    this.onMessage(String(p.text));

                    break;

                case "SetVariable":

                    this.variables[p.name] = Number(p.value);

                    break;

                case "AddVariable":

                    this.variables[p.name] =
                        (this.variables[p.name] || 0) +
                        Number(p.value);

                    break;

                case "MultiplyVariable":

                    this.variables[p.name] =
                        (this.variables[p.name] || 0) *
                        Number(p.value);

                    break;

                case "ShowVariable":

                    this.onMessage(
                        `${p.name}: ${this.variables[p.name] ?? 0}`
                    );

                    break;

                case "Random":

                    this.variables[p.name] =
                        Number(p.min) +
                        Math.random() *
                        (Number(p.max) - Number(p.min));

                    break;

                case "Branch":

                    output =
                        (this.variables[p.name] || 0) >
                        Number(p.greaterThan)
                            ? "true"
                            : "false";

                    break;

                case "Repeat":

                    repeat = Math.max(
                        0,
                        Math.min(100, Math.floor(Number(p.count)))
                    );

                    break;
            }

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

        if (queue.length) {

            this.running = false;

            this.onMessage(
                "Blueprint stopped: execution limit exceeded."
            );
        }
    }
}

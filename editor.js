
(async function () {

    const $ = id => document.getElementById(id);

    const clone = data =>
        JSON.parse(JSON.stringify(data));

    const uid = () => crypto.randomUUID();

    let selected = null;
    let selectedNode = null;
    let pending = null;

    let undoStack = [];
    let redoStack = [];

    let playing = false;
    let busy = false;

    function createEntity(name, type) {

        return {
            id: uid(),
            name,
            type,
            position: [0, 1, 0],
            rotation: [0, 0, 0],
            scale: [1, 1, 1],
            color: "#bcbcc4",
            metalness: 0,
            roughness: 0.5,
            texture: null,
            blueprint: {
                nodes: [],
                links: []
            }
        };
    }

    function newProject() {

        const ground = createEntity("Ground", "box");

        ground.position = [0, -.5, 0];
        ground.scale = [20, 1, 20];
        ground.color = "#777b76";

        const camera = createEntity("Camera", "camera");

        camera.position = [0, 4, 10];
        camera.rotation = [-15, 0, 0];

        return {
            version: 2,
            name: "Untitled Project",

            settings: {
                quality: "high",
                sky: "#8c9cad",
                sun: 2,
                exposure: 1
            },

            assets: [],

            entities: [ground, camera]
        };
    }

    let project = newProject();

    function log(message) {

        $("status").textContent = message;

        const output = $("consoleOutput");

        output.textContent +=
            `[${new Date().toLocaleTimeString()}] ${message}\n`;

        output.scrollTop = output.scrollHeight;
    }

    let engine;

    try {

        engine = await GX.Runtime.create(
            $("canvas"),
            project,
            true
        );

    } catch (error) {

        $("renderInfo").textContent =
            "Renderer initialization failed";

        log(error.message);

        return;
    }

    engine.onMessage = message => {

        $("hud").textContent = message;

        log(message);
    };

    function current() {

        return project.entities.find(
            entity => entity.id === selected
        );
    }

    function graph() {

        return current()?.blueprint;
    }

    function checkpoint() {

        undoStack.push(clone(project));

        if (undoStack.length > 30) {
            undoStack.shift();
        }

        redoStack = [];
    }

    async function refreshWorld() {

        if (playing || busy) return;

        busy = true;

        try {

            await engine.load(project);

            engine.selected = selected;

        } catch (error) {

            log(error.message);

        } finally {

            busy = false;

        }
    }

    async function refresh() {

        await refreshWorld();

        hierarchy();
        inspector();
        drawGraph();
        renderAssets();

        $("projectTitle").textContent = project.name;
        $("projectName").value = project.name;

        for (const key of [
            "quality", "sky", "sun", "exposure"
        ]) {

            $(key).value = project.settings[key];

        }
    }

    function select(id) {

        selected = id;
        selectedNode = null;
        pending = null;

        engine.selected = id;

        hierarchy();
        inspector();
        drawGraph();
    }

    engine.onSelect = select;

    engine.onTransform = (id, values) => {

        const entity = project.entities.find(
            entity => entity.id === id
        );

        if (!entity) return;

        checkpoint();

        Object.assign(entity, values);

        inspector();
    };

    function hierarchy() {

        const root = $("hierarchy");

        root.replaceChildren();

        const query = $("search").value.toLowerCase();

        for (const entity of project.entities) {

            if (!entity.name.toLowerCase().includes(query)) {
                continue;
            }

            const row = document.createElement("div");

            row.className =
                "entity" +
                (selected === entity.id ? " selected" : "");

            row.textContent =
                (entity.type === "camera" ? "◉ " : "◇ ") +
                entity.name;

            row.onclick = () => {
                if (!playing) select(entity.id);
            };

            root.append(row);
        }
    }

    $("search").oninput = hierarchy;

    function inspector() {

        const root = $("inspector");

        root.replaceChildren();

        const entity = current();

        if (!entity) {

            root.textContent = "No entity selected";

            return;
        }

        const heading = document.createElement("div");

        heading.className = "entity-name";

        heading.textContent = entity.name;

        root.append(heading);

        function component(name) {

            const details = document.createElement("details");

            details.className = "component";
            details.open = true;

            const summary = document.createElement("summary");

            summary.textContent = name;

            details.append(summary);

            root.append(details);

            return details;
        }

        function field(parent, name, value, type, callback) {

            const row = document.createElement("label");

            row.className = "field";

            const input = document.createElement("input");

            input.type = type;
            input.value = value;
            input.step = ".1";

            input.onchange = async () => {

                if (playing) return;

                checkpoint();

                callback(
                    type === "number"
                        ? Number(input.value)
                        : input.value
                );

                await refreshWorld();

                hierarchy();

                heading.textContent = entity.name;
            };

            row.append(
                document.createTextNode(name),
                input
            );

            parent.append(row);
        }

        field(root, "Name", entity.name, "text", value => {

            entity.name = value;

        });

        const transform = component("Transform");

        for (const property of [
            "position", "rotation", "scale"
        ]) {

            const title = document.createElement("div");

            title.className = "small-label";

            title.textContent = property.toUpperCase();

            transform.append(title);

            const labels = document.createElement("div");

            labels.className = "axis-labels";

            for (const axis of ["X", "Y", "Z"]) {

                const span = document.createElement("span");

                span.textContent = axis;

                labels.append(span);
            }

            transform.append(labels);

            const xyz = document.createElement("div");

            xyz.className = "xyz";

            for (let i = 0; i < 3; i++) {

                const input = document.createElement("input");

                input.type = "number";
                input.step = ".1";
                input.value = entity[property][i];

                input.onchange = async () => {

                    if (playing) return;

                    checkpoint();

                    entity[property][i] = Number(input.value);

                    await refreshWorld();
                };

                xyz.append(input);
            }

            transform.append(xyz);
        }

        if (!["empty", "camera", "model"].includes(entity.type)) {

            const material = component("Material");

            field(
                material,
                "Base Color",
                entity.color,
                "color",
                value => entity.color = value
            );

            field(
                material,
                "Metalness",
                entity.metalness,
                "number",
                value => entity.metalness =
                    Math.min(1, Math.max(0, value))
            );

            field(
                material,
                "Roughness",
                entity.roughness,
                "number",
                value => entity.roughness =
                    Math.min(1, Math.max(0, value))
            );

            const texture = document.createElement("select");

            const none = document.createElement("option");

            none.value = "";
            none.textContent = "No Texture";

            texture.append(none);

            for (const asset of project.assets) {

                if (asset.type !== "texture") continue;

                const option = document.createElement("option");

                option.value = asset.id;
                option.textContent = asset.name;

                texture.append(option);
            }

            texture.value = entity.texture || "";

            texture.onchange = async () => {

                checkpoint();

                entity.texture = texture.value || null;

                await refreshWorld();
            };

            const row = document.createElement("label");

            row.className = "field";

            row.append(
                document.createTextNode("Texture"),
                texture
            );

            material.append(row);
        }

        const blueprint = component("Blueprint");

        const button = document.createElement("button");

        button.textContent = "Open Blueprint Editor";

        button.onclick = () => switchTab("blueprint");

        blueprint.append(button);
    }

    // ENTITY CREATION

    for (const button of document.querySelectorAll("[data-add]")) {

        button.onclick = async () => {

            if (playing || busy) return;

            checkpoint();

            const type = button.dataset.add;

            const entity = createEntity(
                type[0].toUpperCase() + type.slice(1),
                type
            );

            if (type === "camera") {

                entity.position = [0, 4, 10];
                entity.rotation = [-15, 0, 0];
            }

            project.entities.push(entity);

            selected = entity.id;

            await refresh();

            log("Created " + entity.name);
        };
    }

    $("duplicate").onclick = async () => {

        if (!current() || playing) return;

        checkpoint();

        const entity = clone(current());

        entity.id = uid();
        entity.name += " Copy";
        entity.position[0] += 2;

        project.entities.push(entity);

        selected = entity.id;

        await refresh();
    };

    $("delete").onclick = async () => {

        if (!selected || playing) return;

        checkpoint();

        project.entities = project.entities.filter(
            entity => entity.id !== selected
        );

        selected = null;

        await refresh();
    };

    $("focus").onclick = () => {

        if (!current() || playing) return;

        engine.target.set(...current().position);

        engine.updateCamera();
    };

    for (const button of document.querySelectorAll("[data-tool]")) {

        button.onclick = () => {

            engine.tool = button.dataset.tool;

            document.querySelectorAll("[data-tool]").forEach(
                element => element.classList.toggle(
                    "active",
                    element === button
                )
            );
        };
    }

    $("gridToggle").onclick = () => {

        engine.gridVisible = !engine.gridVisible;

        $("gridToggle").classList.toggle(
            "active",
            engine.gridVisible
        );
    };

    // PROJECT SETTINGS

    for (const key of [
        "quality", "sky", "sun", "exposure"
    ]) {

        $(key).onchange = async event => {

            checkpoint();

            project.settings[key] =
                event.target.type === "number"
                    ? Number(event.target.value)
                    : event.target.value;

            await refreshWorld();
        };
    }

    $("projectName").onchange = event => {

        checkpoint();

        project.name = event.target.value || "Untitled Project";

        $("projectTitle").textContent = project.name;
    };

    // CONTENT BROWSER

    function switchTab(name) {

        document.body.classList.remove("dock-closed");

        $("toggleDock").textContent = "▼";

        document.querySelectorAll(".dock-page").forEach(
            page => page.classList.toggle(
                "active",
                page.id === name
            )
        );

        document.querySelectorAll("[data-tab]").forEach(
            button => button.classList.toggle(
                "active",
                button.dataset.tab === name
            )
        );

        requestAnimationFrame(() => {

            engine.resize();

            if (name === "blueprint") drawGraph();

        });
    }

    for (const button of document.querySelectorAll("[data-tab]")) {

        button.onclick = () => switchTab(button.dataset.tab);

    }

    $("toggleDock").onclick = () => {

        const closed = document.body.classList.toggle(
            "dock-closed"
        );

        $("toggleDock").textContent = closed ? "▲" : "▼";

        requestAnimationFrame(() => engine.resize());
    };

    $("clearConsole").onclick = () => {
        $("consoleOutput").textContent = "";
    };

    function renderAssets() {

        const root = $("assetList");

        root.replaceChildren();

        for (const type of [
            "box", "sphere", "cylinder", "cone", "plane"
        ]) {

            const div = document.createElement("div");

            div.className = "asset";

            const icon = document.createElement("div");

            icon.className = "asset-icon";

            icon.textContent =
                type === "sphere" ? "●" :
                type === "cone" ? "△" : "◇";

            const label = document.createElement("div");

            label.className = "asset-name";

            label.textContent = type;

            div.append(icon, label);

            div.ondblclick = () => {

                document.querySelector(
                    `[data-add="${type}"]`
                )?.click();
            };

            root.append(div);
        }

        for (const asset of project.assets) {

            const div = document.createElement("div");

            div.className = "asset";

            if (asset.type === "texture") {

                const image = document.createElement("img");

                image.src = asset.data;

                div.append(image);

            } else {

                const icon = document.createElement("div");

                icon.className = "asset-icon";
                icon.textContent = "⬡";

                div.append(icon);
            }

            const label = document.createElement("div");

            label.className = "asset-name";
            label.textContent = asset.name;

            div.append(label);

            div.ondblclick = async () => {

                if (playing) return;

                checkpoint();

                if (asset.type === "model") {

                    const entity = createEntity(
                        asset.name,
                        "model"
                    );

                    entity.asset = asset.id;

                    project.entities.push(entity);

                    selected = entity.id;

                } else if (current()) {

                    current().texture = asset.id;

                }

                await refresh();
            };

            root.append(div);
        }
    }

    function readFile(file) {

        return new Promise((resolve, reject) => {

            const reader = new FileReader();

            reader.onload = () => resolve(reader.result);
            reader.onerror = reject;

            reader.readAsDataURL(file);
        });
    }

    $("importAsset").onclick = () => $("modelFile").click();

    $("importTexture").onclick = () => $("textureFile").click();

    async function importFile(event, type) {

        const file = event.target.files[0];

        if (!file || playing) return;

        try {

            const data = await readFile(file);

            checkpoint();

            project.assets.push({
                id: uid(),
                type,
                name: file.name,
                data
            });

            renderAssets();

            log("Imported " + file.name);

        } catch (error) {

            log("Import failed: " + error.message);
        }

        event.target.value = "";
    }

    $("modelFile").onchange = event =>
        importFile(event, "model");

    $("textureFile").onchange = event =>
        importFile(event, "texture");

    // BLUEPRINT EDITOR

    for (const [name, definition] of Object.entries(GX.Nodes)) {

        const option = document.createElement("option");

        option.value = name;

        option.textContent =
            definition.category + ": " + name;

        $("nodeType").append(option);
    }

    $("addNode").onclick = () => {

        if (!graph() || playing) return;

        checkpoint();

        const type = $("nodeType").value;

        graph().nodes.push({
            id: uid(),
            type,
            params: clone(GX.Nodes[type].defaults),
            x: $("graphScroll").scrollLeft + 30,
            y: $("graphScroll").scrollTop + 30
        });

        drawGraph();
    };

    $("deleteNode").onclick = () => {

        if (!graph() || !selectedNode || playing) return;

        checkpoint();

        graph().nodes = graph().nodes.filter(
            node => node.id !== selectedNode
        );

        graph().links = graph().links.filter(
            link =>
                link.from !== selectedNode &&
                link.to !== selectedNode
        );

        selectedNode = null;
        pending = null;

        drawGraph();
    };

    $("disconnect").onclick = () => {

        if (!graph() || !selectedNode || playing) return;

        checkpoint();

        graph().links = graph().links.filter(
            link =>
                link.from !== selectedNode &&
                link.to !== selectedNode
        );

        pending = null;

        drawGraph();
    };

    function drawGraph() {

        const root = $("graph");

        root.querySelectorAll(".node").forEach(
            node => node.remove()
        );

        $("wires").replaceChildren();

        $("graphName").textContent =
            current()?.name || "Select an entity";

        if (!graph()) return;

        for (const node of graph().nodes) {

            const def = GX.Nodes[node.type];

            if (!def) continue;

            const div = document.createElement("div");

            div.className =
                "node " +
                def.category.toLowerCase() +
                (selectedNode === node.id ? " selected" : "");

            div.dataset.node = node.id;

            div.style.left = node.x + "px";
            div.style.top = node.y + "px";

            const header = document.createElement("div");

            header.className = "node-header";
            header.textContent = node.type;

            const body = document.createElement("div");

            body.className = "node-body";

            if (def.category !== "Event") {

                const row = document.createElement("div");

                row.className = "portrow";

                const port = document.createElement("button");

                port.className = "port";
                port.dataset.input = node.id;

                port.onclick = () => {

                    if (!pending || playing) return;

                    if (pending.from === node.id) return;

                    checkpoint();

                    const exists = graph().links.some(link =>
                        link.from === pending.from &&
                        link.output === pending.output &&
                        link.to === node.id
                    );

                    if (!exists) {

                        graph().links.push({
                            ...pending,
                            to: node.id
                        });
                    }

                    pending = null;

                    drawGraph();
                };

                row.append(
                    port,
                    document.createTextNode("Execute")
                );

                body.append(row);
            }

            for (const [key, value] of Object.entries(node.params)) {

                const row = document.createElement("label");

                row.className = "param";

                const label = document.createElement("span");

                label.textContent = key;

                const input = document.createElement("input");

                input.type =
                    typeof value === "number" ? "number" :
                    key === "color" ? "color" : "text";

                input.value = value;
                input.step = ".1";

                input.onchange = () => {

                    checkpoint();

                    node.params[key] =
                        input.type === "number"
                            ? Number(input.value)
                            : input.value;
                };

                row.append(label, input);

                body.append(row);
            }

            const outputs =
                node.type === "Branch"
                    ? ["true", "false"]
                    : ["next"];

            for (const output of outputs) {

                const row = document.createElement("div");

                row.className = "portrow";

                row.style.justifyContent = "flex-end";

                const port = document.createElement("button");

                port.className = "port";

                port.dataset.output =
                    node.id + ":" + output;

                if (
                    pending?.from === node.id &&
                    pending.output === output
                ) {

                    port.classList.add("pending");

                }

                port.onclick = () => {

                    if (playing) return;

                    pending = {
                        from: node.id,
                        output
                    };

                    drawGraph();

                    $("status").textContent =
                        "Select a Blueprint input port.";
                };

                row.append(
                    document.createTextNode(output),
                    port
                );

                body.append(row);
            }

            div.append(header, body);

            root.append(div);

            header.onpointerdown = event => {

                if (playing) return;

                checkpoint();

                selectedNode = node.id;

                const startX = event.clientX;
                const startY = event.clientY;

                const x = node.x;
                const y = node.y;

                header.setPointerCapture(event.pointerId);

                root.querySelectorAll(".node").forEach(
                    element => element.classList.toggle(
                        "selected",
                        element === div
                    )
                );

                header.onpointermove = event => {

                    node.x = Math.max(
                        0,
                        x + event.clientX - startX
                    );

                    node.y = Math.max(
                        0,
                        y + event.clientY - startY
                    );

                    div.style.left = node.x + "px";
                    div.style.top = node.y + "px";

                    drawWires();
                };

                header.onpointerup =
                header.onpointercancel = () => {

                    header.onpointermove = null;

                };
            };
        }

        drawWires();
    }

    function drawWires() {

        const root = $("graph");

        const svg = $("wires");

        const rect = root.getBoundingClientRect();

        svg.replaceChildren();

        if (!graph()) return;

        for (const link of graph().links) {

            const a = root.querySelector(
                `[data-output="${link.from}:${link.output}"]`
            );

            const b = root.querySelector(
                `[data-input="${link.to}"]`
            );

            if (!a || !b) continue;

            const ar = a.getBoundingClientRect();
            const br = b.getBoundingClientRect();

            const x1 = ar.left + ar.width / 2 - rect.left;
            const y1 = ar.top + ar.height / 2 - rect.top;

            const x2 = br.left + br.width / 2 - rect.left;
            const y2 = br.top + br.height / 2 - rect.top;

            const curve = Math.max(
                60,
                Math.abs(x2 - x1) * .5
            );

            const wire = document.createElementNS(
                "http://www.w3.org/2000/svg",
                "path"
            );

            wire.setAttribute(
                "d",
                `M ${x1} ${y1}
                 C ${x1 + curve} ${y1}
                   ${x2 - curve} ${y2}
                   ${x2} ${y2}`
            );

            wire.setAttribute("fill", "none");

            wire.setAttribute(
                "stroke",
                link.output === "true" ? "#70cf99" :
                link.output === "false" ? "#eb858b" :
                "#dedde7"
            );

            wire.setAttribute("stroke-width", "2.5");

            svg.append(wire);
        }
    }

    // PLAY MODE

    function playMode(value) {

        playing = value;

        document.body.classList.toggle(
            "playing",
            value
        );

        document.querySelectorAll(
            "button,input,select"
        ).forEach(element => {

            element.disabled = value;

        });

        $("stop").disabled = !value;
        $("play").disabled = value;
    }

    $("play").onclick = async () => {

        if (playing || busy) return;

        try {

            await engine.load(project);

            playMode(true);

            engine.play();

            log("Game started.");

        } catch (error) {

            log(error.message);

            playMode(false);
        }
    };

    $("stop").onclick = async () => {

        await engine.stop();

        playMode(false);

        $("hud").textContent = "";

        await refresh();

        log("Game stopped.");
    };

    // PROJECT MANAGEMENT

    $("new").onclick = async () => {

        if (!confirm("Create a new project?")) return;

        checkpoint();

        project = newProject();

        selected = null;

        await refresh();

        log("New project created.");
    };

    $("save").onclick = async () => {

        try {

            const result = await desktop.save(project);

            if (result) log("Saved: " + result);

        } catch (error) {

            log(error.message);
        }
    };

    $("open").onclick = async () => {

        try {

            const data = await desktop.open();

            if (!data) return;

            if (!Array.isArray(data.entities)) {

                throw new Error("Invalid project file.");
            }

            checkpoint();

            project = data;

            project.assets ||= [];

            selected = null;

            await refresh();

            log("Project opened.");

        } catch (error) {

            log(error.message);
        }
    };

    $("undo").onclick = async () => {

        if (!undoStack.length || playing) return;

        redoStack.push(clone(project));

        project = undoStack.pop();

        selected = null;

        await refresh();
    };

    $("redo").onclick = async () => {

        if (!redoStack.length || playing) return;

        undoStack.push(clone(project));

        project = redoStack.pop();

        selected = null;

        await refresh();
    };

    // EXPORT

    for (const button of document.querySelectorAll("[data-export]")) {

        button.onclick = async () => {

            if (playing) return;

            button.disabled = true;

            try {

                const format = button.dataset.export;

                log("Exporting " + format.toUpperCase());

                const result = await desktop.export(
                    format,
                    project
                );

                if (result) {

                    log("Exported: " + result);

                } else {

                    log("Export cancelled.");

                }

            } catch (error) {

                log("Export failed: " + error.message);

                alert(error.message);

            } finally {

                button.disabled = false;
            }
        };
    }

    // SHORTCUTS

    window.addEventListener("keydown", event => {

        if (playing) return;

        if (["INPUT", "TEXTAREA", "SELECT"].includes(
            document.activeElement?.tagName
        )) return;

        const ctrl = event.ctrlKey || event.metaKey;

        if (ctrl && event.code === "KeyS") {

            event.preventDefault();

            $("save").click();
        }

        if (ctrl && event.code === "KeyZ") {

            event.preventDefault();

            event.shiftKey
                ? $("redo").click()
                : $("undo").click();
        }

        if (ctrl && event.code === "KeyD") {

            event.preventDefault();

            $("duplicate").click();
        }

        if (event.code === "Delete") {

            $("delete").click();
        }

        const tools = {
            KeyQ: "select",
            KeyW: "move",
            KeyE: "rotate",
            KeyR: "scale"
        };

        if (tools[event.code] && !ctrl) {

            document.querySelector(
                `[data-tool="${tools[event.code]}"]`
            ).click();
        }

        if (event.code === "KeyF") {

            $("focus").click();
        }
    });

    document.querySelectorAll(".menu button").forEach(button => {

        button.addEventListener("click", () => {

            button.closest(".menu").open = false;

        });
    });

    // RENDER STATISTICS

    let frames = 0;
    let elapsed = 0;

    engine.app.on("update", dt => {

        frames++;
        elapsed += dt;

        if (elapsed >= 1) {

            const fps = Math.round(frames / elapsed);

            const device = engine.app.graphicsDevice;

            $("renderInfo").textContent =
                (device.isWebGPU ? "WebGPU" : "WebGL") +
                " | " +
                device.width + " × " + device.height +
                " | " +
                fps + " FPS";

            frames = 0;
            elapsed = 0;
        }
    });

    await refresh();

    playMode(false);

    log("GXEngine ready.");

    for (const input of document.querySelectorAll("[data-effect]")) {

    input.onchange = () => {

        checkpoint();

        project.settings.effects ||= {};

        project.settings.effects[input.dataset.effect] =
            input.checked;

        engine.project.settings.effects = {
            ...project.settings.effects
        };

        engine.applyGraphics();

    };
}

const gxTools = window.installGXTools({

    project: () => project,

    current: () => current(),

    checkpoint,

    refresh,

    select,

    switchTab,

    replaceProject: data => {

        project = data;

        selected = null;

        selectedNode = null;

        pending = null;

    }

});

const originalInspector = inspector;

inspector = function () {

    originalInspector();

    gxTools.physicsInspector();

};

})();
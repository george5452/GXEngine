
window.installGXTools = function (api) {

    const $ = id => document.getElementById(id);

    const Starter = window.GXStarter;

    const style = document.createElement("style");

    style.textContent = `

    .gx-resource-toolbar {
        display:flex;
        gap:8px;
        padding:8px;
        align-items:center;
        border-bottom:1px solid #393a42;
    }

    .gx-resource-toolbar input {
        flex:1;
    }

    .gx-resource-grid {
        display:grid;
        grid-template-columns:
            repeat(auto-fill,minmax(125px,1fr));
        gap:8px;
        padding:12px;
        overflow:auto;
    }

    .gx-resource-card {
        background:#292a32;
        border:1px solid #41424c;
        border-radius:5px;
        padding:9px;
        text-align:left;
        cursor:pointer;
        min-height:90px;
    }

    .gx-resource-card:hover {
        background:#383641;
        border-color:#ac91f5;
    }

    .gx-resource-icon {
        display:grid;
        place-items:center;
        font-size:29px;
        height:48px;
        color:#bea6ff;
    }

    .gx-resource-name {
        font-weight:600;
        margin:5px 0;
    }

    .gx-resource-description {
        color:#9395a4;
        font-size:10px;
    }

    .gx-material-swatch {
        width:43px;
        height:43px;
        margin:5px auto;
        border-radius:50%;
        box-shadow:
            inset -10px -10px 15px #0005,
            inset 6px 6px 12px #fff3;
    }

    .gx-template-overlay {
        position:fixed;
        inset:0;
        z-index:20000;
        background:#000b;
        display:grid;
        place-items:center;
        backdrop-filter:blur(7px);
    }

    .gx-template-window {
        width:min(850px,90vw);
        background:#23242b;
        border:1px solid #484954;
        border-radius:9px;
        padding:25px;
        box-shadow:0 25px 100px #000b;
    }

    .gx-template-window h1 {
        font-size:25px;
        font-weight:600;
        margin-top:0;
    }

    .gx-template-grid {
        display:grid;
        grid-template-columns:repeat(3,1fr);
        gap:12px;
        margin:24px 0;
    }

    .gx-template {
        padding:15px;
        min-height:145px;
        text-align:left;
        background:#303139;
        border:1px solid #484954;
        border-radius:6px;
    }

    .gx-template:hover {
        border-color:#b99aff;
        background:#393443;
    }

    .gx-template-icon {
        font-size:32px;
        margin-bottom:12px;
        color:#baa2f5;
    }

    .gx-template small {
        display:block;
        color:#a2a4b2;
        margin-top:9px;
        line-height:1.6;
    }

    `;

    document.head.append(style);

    // PHYSICS INSPECTOR

    function physicsInspector() {

        $("gxPhysicsInspector")?.remove();

        const entity = api.current();

        if (!entity) return;

        const panel = document.createElement("details");

        panel.id = "gxPhysicsInspector";
        panel.className = "component";
        panel.open = true;

        const heading = document.createElement("summary");

        heading.textContent = "Physics";

        panel.append(heading);

        entity.physics ||= {

            ...GX.PhysicsDefaults

        };

        function field(name, key, options = null) {

            const row = document.createElement("label");

            row.className = "field";

            const input = document.createElement(

                options ? "select" : "input"

            );

            if (options) {

                for (const value of options) {

                    const option = document.createElement(
                        "option"
                    );

                    option.value = value;

                    option.textContent =

                        value[0].toUpperCase() +
                        value.slice(1);

                    input.append(option);

                }

            } else {

                input.type = "number";
                input.step = "0.05";

            }

            input.value = entity.physics[key];

            input.onchange = () => {

                api.checkpoint();

                entity.physics[key] = options

                    ? input.value

                    : Number(input.value);

                if (key === "body") {

                    physicsInspector();

                }

            };

            row.append(
                document.createTextNode(name),
                input
            );

            panel.append(row);

        }

        field("Body Type", "body", [

            "none",
            "static",
            "dynamic",
            "kinematic",
            "trigger"

        ]);

        field("Collider", "shape", [

            "auto",
            "box",
            "sphere",
            "capsule",
            "cylinder",
            "mesh"

        ]);

        field("Friction", "friction");

        field("Bounciness", "restitution");

        if (entity.physics.body === "dynamic") {

            field("Mass (kg)", "mass");
            field("Gravity Scale", "gravity");
            field("Linear Damping", "linearDamping");
            field("Angular Damping", "angularDamping");

            const row = document.createElement("label");

            row.className = "field";

            const checkbox = document.createElement(
                "input"
            );

            checkbox.type = "checkbox";

            checkbox.checked = entity.physics.ccd;

            checkbox.onchange = () => {

                api.checkpoint();

                entity.physics.ccd = checkbox.checked;

            };

            row.append(

                document.createTextNode(
                    "Continuous Collision"
                ),

                checkbox

            );

            panel.append(row);

        }

        $("inspector").append(panel);

    }

    // STARTER CONTENT TAB

    const tab = document.createElement("button");

    tab.textContent = "Starter Content";

    tab.dataset.tab = "starter";

    $("dockToolbar").insertBefore(

        tab,

        $("dockToolbar").querySelector(".spacer")

    );

    const page = document.createElement("div");

    page.id = "starter";

    page.className = "dock-page";

    page.innerHTML = `

        <div class="gx-resource-toolbar">

            <select id="gxResourceCategory"></select>

            <input id="gxResourceSearch"
                   placeholder="Search starter resources...">

        </div>

        <div id="gxResourceGrid"
             class="gx-resource-grid"></div>

    `;

    $("dock").append(page);

    tab.onclick = () => api.switchTab("starter");

    const category = $("gxResourceCategory");

    for (const name of [

        "All Resources",

        ...Object.keys(Starter.categories),

        "Materials"

    ]) {

        const option = document.createElement("option");

        option.value = name;
        option.textContent = name;

        category.append(option);

    }

    function renderResources() {

        const root = $("gxResourceGrid");

        root.replaceChildren();

        const query = $("gxResourceSearch")
            .value.toLowerCase();

        const filter = category.value;

        const icons = {

            "Level Design": "▧",
            "Physics": "◇",
            "Environment": "♧",
            "Players": "♙",
            "Gameplay": "⚑"

        };

        for (const [group, resources] of Object.entries(

            Starter.categories

        )) {

            if (
                filter !== "All Resources" &&
                filter !== group
            ) continue;

            for (const name of resources) {

                if (!name.toLowerCase().includes(query)) {
                    continue;
                }

                const card = document.createElement("button");

                card.className = "gx-resource-card";

                card.innerHTML = `

                    <div class="gx-resource-icon">
                        ${icons[group]}
                    </div>

                    <div class="gx-resource-name">
                        ${name}
                    </div>

                    <div class="gx-resource-description">
                        ${group} · Add to scene
                    </div>

                `;

                card.onclick = async () => {

                    api.checkpoint();

                    const result = Starter.prefab(name);

                    const entities = Array.isArray(result)
                        ? result
                        : [result];

                    api.project().entities.push(...entities);

                    await api.refresh();

                    api.select(entities[0].id);

                };

                root.append(card);

            }

        }

        if (

            filter === "Materials" ||

            filter === "All Resources"

        ) {

            for (const [name, material] of Object.entries(

                Starter.materials

            )) {

                if (!name.toLowerCase().includes(query)) {
                    continue;
                }

                const card = document.createElement("button");

                card.className = "gx-resource-card";

                card.innerHTML = `

                    <div class="gx-material-swatch"
                         style="background:${material.color}">
                    </div>

                    <div class="gx-resource-name">
                        ${name}
                    </div>

                    <div class="gx-resource-description">
                        Apply to selected object
                    </div>

                `;

                card.onclick = async () => {

                    const entity = api.current();

                    if (!entity) {

                        alert("Select an object first.");

                        return;

                    }

                    api.checkpoint();

                    Object.assign(entity, material);

                    await api.refresh();

                };

                root.append(card);

            }

        }

    }

    category.onchange = renderResources;

    $("gxResourceSearch").oninput = renderResources;

    renderResources();

    // PROJECT TEMPLATE WINDOW

    function showTemplates() {

        const overlay = document.createElement("div");

        overlay.className = "gx-template-overlay";

        const window = document.createElement("div");

        window.className = "gx-template-window";

        window.innerHTML = `

            <h1>Create a project</h1>

            <div style="color:#a6a8b6">
                Choose a starting point for your game.
            </div>

        `;

        const grid = document.createElement("div");

        grid.className = "gx-template-grid";

        const templates = [

            [
                "Blank",
                "◇",
                "An empty scene for custom projects."
            ],

            [
                "First Person",
                "◎",
                "Immersive exploration with collision and movement."
            ],

            [
                "Third Person",
                "♙",
                "Third-person adventure and exploration."
            ],

            [
                "Platformer",
                "▥",
                "Jumping challenges, collectibles and finish zones."
            ],

            [
                "Physics Sandbox",
                "▦",
                "Interactive rigid bodies, gravity and physics props."
            ]

        ];

        for (const [name, icon, description] of templates) {

            const button = document.createElement("button");

            button.className = "gx-template";

            button.innerHTML = `

                <div class="gx-template-icon">${icon}</div>

                <strong>${name}</strong>

                <small>${description}</small>

            `;

            button.onclick = async () => {

                api.checkpoint();

                api.replaceProject(

                    Starter.template(name)

                );

                overlay.remove();

                await api.refresh();

            };

            grid.append(button);

        }

        const cancel = document.createElement("button");

        cancel.textContent = "Cancel";

        cancel.onclick = () => overlay.remove();

        window.append(grid, cancel);

        overlay.append(window);

        document.body.append(overlay);

    }

    $("new").onclick = () => {

        if (!confirm(
            "Create a new project? Save your current work first."
        )) return;

        showTemplates();

    };

    return {

        physicsInspector,

        showTemplates

    };

};

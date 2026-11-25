
(() => {

    const steps = [

        {
            target: "#hierarchyPanel",
            title: "Scene Hierarchy",
            text: "Every object in your game appears here. Select an object to edit its properties."
        },

        {
            target: "#viewport",
            title: "3D Workspace",
            text: "Right-drag to orbit, middle-drag to pan and scroll to zoom. Click objects to select them."
        },

        {
            target: "#toolbar",
            title: "Transform Tools",
            text: "Use Q, W, E and R to select, move, rotate and scale. Drag the colored 3D gizmos to make precise adjustments."
        },

        {
            target: "#inspectorPanel",
            title: "Inspector",
            text: "Change object properties, materials, rendering effects and environment settings here."
        },

        {
            target: "#hierarchyPanel",
            title: "Create a Player",
            text: "Open the Scene menu and add a First Person Player or Third Person Player. The engine automatically provides movement and collision."
        },

        {
            target: "#dock",
            title: "Content Browser",
            text: "Import GLB models and textures. Double-click models to insert them into your scene."
        },

        {
            target: '[data-tab="blueprint"]',
            title: "Visual Scripting",
            text: "Connect Blueprint nodes to create gameplay logic. Events start actions, and variables store information."
        },

        {
            target: "#play",
            title: "Play Your Game",
            text: "Press Play to test your game. Click the viewport to control the player. WASD moves, Shift sprints, Space jumps, and Escape releases the mouse."
        },

        {
            target: '[data-export="exe"].primary',
            title: "Publish Your Game",
            text: "Export a standalone Windows executable. HTML and ZIP exports are also available in the File menu."
        }

    ];

    let index = 0;
    let overlay = null;
    let highlighted = null;

    const style = document.createElement("style");

    style.textContent = `
        .gx-tour-overlay {
            position: fixed;
            inset: 0;
            background: rgba(0,0,0,.52);
            z-index: 10000;
        }

        .gx-tour-highlight {
            outline: 2px solid #b99aff !important;
            outline-offset: -2px;
        }

        .gx-tour-card {
            position: fixed;
            width: min(360px, calc(100vw - 24px));
            background: #282933;
            border: 1px solid #76629f;
            color: #eee;
            border-radius: 10px;
            box-shadow: 0 15px 60px #0009;
            padding: 22px;
            z-index: 10001;
            font: 13px system-ui;
        }

        .gx-tour-card h2 {
            font-size: 20px;
            margin: 12px 0;
        }

        .gx-tour-card p {
            line-height: 1.7;
            color: #bec0cc;
        }

        .gx-tour-progress {
            color: #bda3ff;
            font-size: 11px;
        }

        .gx-tour-actions {
            display: flex;
            gap: 7px;
            margin-top: 20px;
        }
    `;

    document.head.append(style);

    function close() {

        highlighted?.classList.remove("gx-tour-highlight");

        overlay?.remove();

        document.querySelector(".gx-tour-card")?.remove();

        overlay = null;

    }

    function show() {

        close();

        const step = steps[index];

        const target = document.querySelector(step.target);

        overlay = document.createElement("div");

        overlay.className = "gx-tour-overlay";

        document.body.append(overlay);

        const card = document.createElement("div");

        card.className = "gx-tour-card";

        const progress = document.createElement("div");

        progress.className = "gx-tour-progress";

        progress.textContent =
            `GETTING STARTED • ${index + 1} / ${steps.length}`;

        const heading = document.createElement("h2");

        heading.textContent = step.title;

        const paragraph = document.createElement("p");

        paragraph.textContent = step.text;

        const actions = document.createElement("div");

        actions.className = "gx-tour-actions";

        const skip = document.createElement("button");

        skip.textContent = "Exit Tour";

        skip.onclick = close;

        const previous = document.createElement("button");

        previous.textContent = "Back";

        previous.disabled = index === 0;

        previous.onclick = () => {

            index--;

            show();

        };

        const next = document.createElement("button");

        next.className = "primary";

        next.textContent = index === steps.length - 1
            ? "Finish"
            : "Next";

        next.onclick = () => {

            if (index >= steps.length - 1) {

                close();

            } else {

                index++;

                show();

            }

        };

        actions.append(skip, previous, next);

        card.append(progress, heading, paragraph, actions);

        document.body.append(card);

        const rect = target?.getBoundingClientRect();

        if (rect) {

            highlighted = target;

            target.classList.add("gx-tour-highlight");

        }

        const width = card.offsetWidth;
        const height = card.offsetHeight;

        const preferredX = rect
            ? rect.right + 12
            : innerWidth / 2 - width / 2;

        const preferredY = rect
            ? rect.top
            : innerHeight / 2 - height / 2;

        card.style.left = Math.max(

            12,

            Math.min(
                innerWidth - width - 12,
                preferredX
            )

        ) + "px";

        card.style.top = Math.max(

            12,

            Math.min(
                innerHeight - height - 12,
                preferredY
            )

        ) + "px";

    }

    document.getElementById("startTour").onclick = () => {

        index = 0;

        show();

    };

    document.addEventListener("keydown", event => {

        if (event.key === "Escape" && overlay) {

            close();

        }

    });

})();

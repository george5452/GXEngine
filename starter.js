
window.GXStarter = (() => {

    const uid = () => crypto.randomUUID();

    const materials = {

        Concrete: {
            color: "#989a96",
            metalness: 0,
            roughness: 0.9
        },

        Grass: {
            color: "#52743d",
            metalness: 0,
            roughness: 0.95
        },

        Wood: {
            color: "#96704b",
            metalness: 0,
            roughness: 0.75
        },

        Steel: {
            color: "#a3a9b4",
            metalness: 0.95,
            roughness: 0.28
        },

        Gold: {
            color: "#e8b94e",
            metalness: 1,
            roughness: 0.22
        },

        Rubber: {
            color: "#303238",
            metalness: 0,
            roughness: 0.85
        },

        Plastic: {
            color: "#e05749",
            metalness: 0,
            roughness: 0.35
        },

        Stone: {
            color: "#696c70",
            metalness: 0,
            roughness: 0.98
        },

        Sand: {
            color: "#d6bd8e",
            metalness: 0,
            roughness: 1
        },

        Snow: {
            color: "#edf3ff",
            metalness: 0,
            roughness: 0.8
        }

    };

    function object(
        name,
        type = "box",
        position = [0, 1, 0],
        scale = [1, 1, 1],
        material = "Concrete"
    ) {

        return {

            id: uid(),

            name,
            type,
            position,
            scale,

            rotation: [0, 0, 0],

            ...materials[material],

            texture: null,

            physics: {
                body: "static",
                shape: "auto",
                mass: 1,
                friction: 0.7,
                restitution: 0.1,
                gravity: 1,
                linearDamping: 0.05,
                angularDamping: 0.05,
                ccd: true
            },

            blueprint: {
                nodes: [],
                links: []
            }

        };

    }

    function blueprint(entity, definitions) {

        const nodes = definitions.map((def, i) => ({

            id: uid(),

            type: def[0],

            params: def[1] || {},

            x: 30 + i * 240,

            y: 50

        }));

        entity.blueprint = {

            nodes,

            links: nodes.slice(0, -1).map((node, i) => ({

                from: node.id,

                to: nodes[i + 1].id,

                output: "next"

            }))

        };

        return entity;

    }

    function prefab(name) {

        let e;

        switch (name) {

            case "Floor":

                return object(
                    "Floor",
                    "box",
                    [0, -0.25, 0],
                    [5, 0.5, 5]
                );

            case "Wall":

                return object(
                    "Wall",
                    "box",
                    [0, 1.5, 0],
                    [5, 3, 0.3]
                );

            case "Pillar":

                return object(
                    "Pillar",
                    "cylinder",
                    [0, 1.5, 0],
                    [0.6, 3, 0.6]
                );

            case "Ramp":

                e = object(
                    "Ramp",
                    "box",
                    [0, 0.8, 0],
                    [3, 0.25, 5]
                );

                e.rotation = [-18, 0, 0];

                return e;

            case "Platform":

                return object(
                    "Platform",
                    "box",
                    [0, 2, 0],
                    [3, 0.4, 3]
                );

            case "Crate":

                e = object(
                    "Physics Crate",
                    "box",
                    [0, 2, 0],
                    [1, 1, 1],
                    "Wood"
                );

                e.physics.body = "dynamic";
                e.physics.mass = 8;

                return e;

            case "Bouncy Ball":

                e = object(
                    "Bouncy Ball",
                    "sphere",
                    [0, 4, 0],
                    [1, 1, 1],
                    "Plastic"
                );

                e.physics.body = "dynamic";
                e.physics.restitution = 0.85;

                return e;

            case "Barrel":

                e = object(
                    "Barrel",
                    "cylinder",
                    [0, 1, 0],
                    [0.8, 1.3, 0.8],
                    "Steel"
                );

                e.physics.body = "dynamic";
                e.physics.mass = 12;

                return e;

            case "Rock":

                e = object(
                    "Rock",
                    "sphere",
                    [0, 0.5, 0],
                    [1.8, 1.1, 1.4],
                    "Stone"
                );

                e.rotation = [15, 30, 10];

                return e;

            case "Tree":

                return [

                    object(
                        "Tree Trunk",
                        "cylinder",
                        [0, 1.5, 0],
                        [0.4, 3, 0.4],
                        "Wood"
                    ),

                    object(
                        "Tree Crown",
                        "sphere",
                        [0, 3.4, 0],
                        [2.6, 2.8, 2.6],
                        "Grass"
                    )

                ];

            case "First Person":

                return object(

                    "First Person Player",

                    "player-first",

                    [0, 1.1, 0],

                    [1, 1, 1],

                    "Plastic"

                );

            case "Third Person":

                return object(

                    "Third Person Player",

                    "player-third",

                    [0, 1.1, 0],

                    [1, 1, 1],

                    "Plastic"

                );

            case "Collectible":

                e = object(

                    "Collectible",

                    "sphere",

                    [0, 1, 0],

                    [0.5, 0.5, 0.5],

                    "Gold"

                );

                e.physics.body = "trigger";

                return blueprint(e, [

                    ["TriggerEnter"],

                    ["AddVariable", {
                        name: "score",
                        value: 1
                    }],

                    ["ShowVariable", {
                        name: "score"
                    }],

                    ["DestroySelf"]

                ]);

            case "Checkpoint":

                e = object(

                    "Checkpoint",

                    "box",

                    [0, 0.1, 0],

                    [2, 0.2, 2],

                    "Grass"

                );

                e.physics.body = "trigger";
                e.color = "#3ce6a0";

                return blueprint(e, [

                    ["TriggerEnter"],

                    ["SetCheckpoint"]

                ]);

            case "Finish Zone":

                e = object(

                    "Finish Zone",

                    "box",

                    [0, 0.15, 0],

                    [3, 0.3, 3]

                );

                e.color = "#72abff";

                e.physics.body = "trigger";

                return blueprint(e, [

                    ["TriggerEnter"],

                    ["FinishLevel", {
                        message: "Level Complete!"
                    }]

                ]);

            case "Physics Launcher":

                e = prefab("Bouncy Ball");

                e.name = "Physics Launcher";

                return blueprint(e, [

                    ["KeyPressed", {
                        key: "KeyE"
                    }],

                    ["ApplyImpulse", {
                        x: 0,
                        y: 15,
                        z: -5
                    }]

                ]);

        }

        return object(name);

    }

    const categories = {

        "Level Design": [

            "Floor",
            "Wall",
            "Pillar",
            "Ramp",
            "Platform"

        ],

        "Physics": [

            "Crate",
            "Bouncy Ball",
            "Barrel",
            "Physics Launcher"

        ],

        "Environment": [

            "Tree",
            "Rock"

        ],

        "Players": [

            "First Person",
            "Third Person"

        ],

        "Gameplay": [

            "Collectible",
            "Checkpoint",
            "Finish Zone"

        ]

    };

    function template(name) {

        const project = {

            version: 2,

            name: "Untitled Project",

            assets: [],

            settings: {

                quality: "cinematic",

                sky: "#a6bacf",

                sun: 2.5,

                exposure: 1,

                effects: {

                    ssao: true,
                    bloom: true,
                    taa: true,
                    vignette: false,
                    fog: false,
                    dof: false

                }

            },

            entities: []

        };

        function add(name, position) {

            const result = prefab(name);

            const entities = Array.isArray(result)
                ? result
                : [result];

            if (position) {

                for (const entity of entities) {

                    entity.position = entity.position.map(
                        (value, i) => value + position[i]
                    );

                }

            }

            project.entities.push(...entities);

        }

        if (name === "Blank") {

            return project;

        }

        project.entities.push(

            object(

                "Ground",

                "box",

                [0, -0.5, 0],

                [40, 1, 40],

                "Concrete"

            )

        );

        add(

            name === "First Person"
                ? "First Person"
                : "Third Person"

        );

        if (name === "First Person") {

            add("Wall", [0, 0, -8]);
            add("Pillar", [-4, 0, -4]);
            add("Pillar", [4, 0, -4]);
            add("Crate", [3, 0, -3]);
            add("Collectible", [0, 0, -6]);
            add("Finish Zone", [0, 0, -10]);

        }

        if (name === "Third Person") {

            add("Tree", [-6, 0, -5]);
            add("Tree", [6, 0, -5]);
            add("Rock", [-4, 0, -3]);
            add("Collectible", [0, 0, -4]);
            add("Finish Zone", [0, 0, -12]);

        }

        if (name === "Platformer") {

            for (let i = 0; i < 8; i++) {

                add("Platform", [

                    0,

                    i * 0.4 - 1.5,

                    -3 - i * 3.5

                ]);

                add("Collectible", [

                    0,

                    i * 0.4 + 0.5,

                    -3 - i * 3.5

                ]);

            }

            add("Checkpoint", [0, 1.8, -13.5]);
            add("Finish Zone", [0, 3.2, -27.5]);

        }

        if (name === "Physics Sandbox") {

            for (let x = 0; x < 4; x++) {

                for (let y = 0; y < 4; y++) {

                    add("Crate", [

                        x * 1.1 - 2,

                        y * 1.1,

                        -6

                    ]);

                }

            }

            add("Ramp", [5, 0, -4]);
            add("Barrel", [-5, 0, -4]);
            add("Bouncy Ball", [3, 2, -4]);
            add("Physics Launcher", [-3, 0, -4]);

        }

        return project;

    }

    return {

        materials,
        categories,
        prefab,
        template

    };

})();

use super::*;

use rhai::{AST, CallFnOptions, Engine, Scope};
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};

// ==========================================
// GAMEPLAY COMMANDS
// ==========================================

enum Action {
    Move([f32; 3]),
    Rotate([f32; 3]),
    Position([f32; 3]),
    Rotation([f32; 3]),
    Scale([f32; 3]),
    Color([f32; 3]),
    Velocity([f32; 3]),
    Impulse([f32; 3]),
    Light(f32),
    Emission(f32),
    LightColor([f32; 3]),
}

#[derive(Default)]
struct GameState {
    actions: Vec<Action>,
    console: Vec<String>,
    held: Vec<String>,
    pressed: Vec<String>,
    position: [f32; 3],
    rotation: [f32; 3],
    player: [f32; 3],
    time: f64,
}

struct ScriptInstance {
    ast: AST,
    scope: Scope<'static>,
    started: bool,
    failed: bool,
    has_start: bool,
    has_update: bool,
}

struct ScriptTab {
    name: String,
    source: String,
    dirty: bool,
}

pub(super) struct GXScripter {
    pub open: bool,

    engine: Engine,
    state: Arc<Mutex<GameState>>,

    instances: HashMap<usize, ScriptInstance>,
    cache: HashMap<String, AST>,

    tabs: Vec<ScriptTab>,
    active: usize,
    new_name: String,
    explorer: bool,
    show_console: bool,

    console: Vec<String>,
    was_playing: bool,
    time: f64,
}

// ==========================================
// SCRIPT API
// ==========================================

impl Default for GXScripter {
    fn default() -> Self {
        let mut engine = Engine::new();

        engine.set_max_operations(50_000);
        engine.set_max_call_levels(32);
        engine.set_max_variables(256);
        engine.set_max_array_size(16384);
        engine.set_max_string_size(65536);

        let state = Arc::new(Mutex::new(GameState::default()));

        macro_rules! vector_api {
            ($name:literal, $action:ident) => {
                let shared = state.clone();

                engine.register_fn($name, move |x: f64, y: f64, z: f64| {
                    let v = [x as f32, y as f32, z as f32];

                    if v.iter().all(|x| x.is_finite()) {
                        shared.lock().unwrap().actions.push(Action::$action(v));
                    }
                });
            };
        }

        vector_api!("move", Move);
        vector_api!("rotate", Rotate);
        vector_api!("set_position", Position);
        vector_api!("set_rotation", Rotation);
        vector_api!("set_scale", Scale);
        vector_api!("set_color", Color);
        vector_api!("set_light_color", LightColor);
        vector_api!("set_velocity", Velocity);
        vector_api!("add_impulse", Impulse);

        let shared = state.clone();

        engine.register_fn("set_light_energy", move |value: f64| {
            if value.is_finite() {
                shared
                    .lock()
                    .unwrap()
                    .actions
                    .push(Action::Light(value as f32));
            }
        });

        let shared = state.clone();

        engine.register_fn("set_emission", move |value: f64| {
            if value.is_finite() {
                shared
                    .lock()
                    .unwrap()
                    .actions
                    .push(Action::Emission(value as f32));
            }
        });

        let shared = state.clone();

        engine.register_fn("key_down", move |key: &str| {
            shared.lock().unwrap().held.contains(&key.to_uppercase())
        });

        let shared = state.clone();

        engine.register_fn("key_pressed", move |key: &str| {
            shared.lock().unwrap().pressed.contains(&key.to_uppercase())
        });

        let shared = state.clone();

        engine.register_fn("position", move || -> rhai::Array {
            shared
                .lock()
                .unwrap()
                .position
                .iter()
                .map(|v| rhai::Dynamic::from(*v as f64))
                .collect()
        });

        let shared = state.clone();

        engine.register_fn("rotation", move || -> rhai::Array {
            shared
                .lock()
                .unwrap()
                .rotation
                .iter()
                .map(|v| rhai::Dynamic::from(*v as f64))
                .collect()
        });

        let shared = state.clone();

        engine.register_fn("player_position", move || -> rhai::Array {
            shared
                .lock()
                .unwrap()
                .player
                .iter()
                .map(|v| rhai::Dynamic::from(*v as f64))
                .collect()
        });

        let shared = state.clone();

        engine.register_fn("distance_to_player", move || -> f64 {
            let s = shared.lock().unwrap();

            Vec3::from_array(s.position).distance(Vec3::from_array(s.player)) as f64
        });

        let shared = state.clone();

        engine.register_fn("time", move || -> f64 { shared.lock().unwrap().time });

        let shared = state.clone();

        engine.on_print(move |message| {
            let mut s = shared.lock().unwrap();

            if s.console.len() < 100 {
                s.console.push(message.into());
            }
        });

        Self {
            open: false,

            engine,
            state,

            instances: HashMap::new(),
            cache: HashMap::new(),

            tabs: Vec::new(),
            active: 0,

            new_name: "new_script".into(),
            explorer: true,
            show_console: true,

            console: vec!["GXScript Development Runtime".into()],

            was_playing: false,
            time: 0.0,
        }
    }
}

// ==========================================
// GXS LANGUAGE TRANSLATION
// ==========================================

fn translate(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();

    let mut output = String::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '"' || c == '\'' {
            let quote = c;

            output.push(c);
            i += 1;

            while i < chars.len() {
                let ch = chars[i];

                output.push(ch);
                i += 1;

                if ch == '\\' && i < chars.len() {
                    output.push(chars[i]);
                    i += 1;
                    continue;
                }

                if ch == quote {
                    break;
                }
            }

            continue;
        }

        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() {
                let ch = chars[i];

                output.push(ch);
                i += 1;

                if ch == '\n' {
                    break;
                }
            }

            continue;
        }

        if c == '/' && chars.get(i + 1) == Some(&'*') {
            output.push_str("/*");
            i += 2;

            let mut depth = 1;

            while i < chars.len() && depth > 0 {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    output.push_str("/*");
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    output.push_str("*/");
                    i += 2;
                } else {
                    output.push(chars[i]);
                    i += 1;
                }
            }

            continue;
        }

        if c.is_alphabetic() || c == '_' {
            let start = i;

            i += 1;

            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }

            let word: String = chars[start..i].iter().collect();

            match word.as_str() {
                "func" => output.push_str("fn"),
                "var" => output.push_str("let"),
                _ => output.push_str(&word),
            }

            continue;
        }

        output.push(c);
        i += 1;
    }

    output
}

fn valid_filename(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".gxs") else {
        return false;
    };

    !stem.is_empty()
        && stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

// ==========================================
// FILE MANAGEMENT
// ==========================================

impl GXScripter {
    fn log(&mut self, text: String) {
        self.console.push(text);

        if self.console.len() > 300 {
            self.console.remove(0);
        }
    }

    pub(super) fn save_pending(&mut self, project: &std::path::Path) -> Result<(), String> {
        let directory = project.join("Assets/Scripts");

        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;

        for tab in &mut self.tabs {
            if !tab.dirty {
                continue;
            }

            fs::write(directory.join(&tab.name), &tab.source).map_err(|e| e.to_string())?;

            tab.dirty = false;
        }

        Ok(())
    }

    fn open_script(&mut self, project: &std::path::Path, name: &str) -> Result<(), String> {
        if !valid_filename(name) {
            return Err("Invalid script name".into());
        }

        if let Some(index) = self.tabs.iter().position(|tab| tab.name == name) {
            self.active = index;
            self.open = true;
            return Ok(());
        }

        let source = fs::read_to_string(project.join("Assets/Scripts").join(name))
            .map_err(|e| e.to_string())?;

        self.tabs.push(ScriptTab {
            name: name.into(),
            source,
            dirty: false,
        });

        self.active = self.tabs.len() - 1;
        self.open = true;

        Ok(())
    }

    fn create_script(&mut self, project: &std::path::Path, name: &str) -> Result<String, String> {
        let name = format!("{}.gxs", name.trim());

        if !valid_filename(&name) {
            return Err("Invalid script name".into());
        }

        let directory = project.join("Assets/Scripts");

        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;

        let path = directory.join(&name);

        if path.exists() {
            return Err("Script already exists".into());
        }

        let source = concat!(
            "// GXScript component\n\n",
            "func start() {\n",
            "    print(\"Ready!\");\n",
            "}\n\n",
            "func update(dt) {\n",
            "    \n",
            "}\n"
        );

        fs::write(path, source).map_err(|e| e.to_string())?;

        self.open_script(project, &name)?;

        Ok(name)
    }
}

// ==========================================
// INPUT
// ==========================================

fn capture_keys(ctx: &egui::Context, state: &mut GameState, enabled: bool) {
    state.held.clear();
    state.pressed.clear();

    if !enabled {
        return;
    }

    use egui::Key::*;

    let keys = [
        ("A", A),
        ("B", B),
        ("C", C),
        ("D", D),
        ("E", E),
        ("F", F),
        ("G", G),
        ("H", H),
        ("I", I),
        ("J", J),
        ("K", K),
        ("L", L),
        ("M", M),
        ("N", N),
        ("O", O),
        ("P", P),
        ("Q", Q),
        ("R", R),
        ("S", S),
        ("T", T),
        ("U", U),
        ("V", V),
        ("W", W),
        ("X", X),
        ("Y", Y),
        ("Z", Z),
        ("SPACE", Space),
        ("ENTER", Enter),
        ("UP", ArrowUp),
        ("DOWN", ArrowDown),
        ("LEFT", ArrowLeft),
        ("RIGHT", ArrowRight),
    ];

    ctx.input(|input| {
        for (name, key) in keys {
            if input.key_down(key) {
                state.held.push(name.into());
            }

            if input.key_pressed(key) {
                state.pressed.push(name.into());
            }
        }

        if input.modifiers.shift {
            state.held.push("SHIFT".into());
        }

        if input.modifiers.ctrl {
            state.held.push("CTRL".into());
        }

        if input.pointer.primary_down() {
            state.held.push("MOUSE1".into());
        }

        if input.pointer.primary_pressed() {
            state.pressed.push("MOUSE1".into());
        }
    });
}

// ==========================================
// GAMEPLAY EXECUTION
// ==========================================

impl GXEngine {
    pub(super) fn scripting_update(&mut self, ctx: &egui::Context, dt: f32) {
        if !self.playing {
            if self.scripter.was_playing {
                self.scripter.instances.clear();
                self.scripter.cache.clear();

                self.scripter.was_playing = false;

                self.scripter.log("[GXS] Game stopped".into());
            }

            return;
        }

        if !self.scripter.was_playing {
            self.scripter.was_playing = true;

            self.scripter.instances.clear();
            self.scripter.cache.clear();
            self.scripter.time = 0.0;

            self.scripter.log("[GXS] Game started".into());

            let count = self.scene.objects.len();

            for index in 0..count {
                let name = self.scene.objects[index].script.clone();

                if name.is_empty() {
                    continue;
                }

                if !valid_filename(&name) {
                    self.scripter
                        .log(format!("[ERROR] Invalid script: {}", name));

                    continue;
                }

                let ast = if let Some(ast) = self.scripter.cache.get(&name) {
                    ast.clone()
                } else {
                    let path = self.project_dir.join("Assets/Scripts").join(&name);

                    let source = match fs::read_to_string(path) {
                        Ok(source) => source,

                        Err(error) => {
                            self.scripter.log(format!("[{}] {}", name, error));

                            continue;
                        }
                    };

                    match self.scripter.engine.compile(translate(&source)) {
                        Ok(ast) => {
                            self.scripter.cache.insert(name.clone(), ast.clone());

                            ast
                        }

                        Err(error) => {
                            self.scripter.log(format!("[{}] {}", name, error));

                            continue;
                        }
                    }
                };

                let has_start = ast
                    .iter_functions()
                    .any(|f| f.name == "start" && f.params.is_empty());

                let has_update = ast
                    .iter_functions()
                    .any(|f| f.name == "update" && f.params.len() == 1);

                self.scripter.instances.insert(
                    index,
                    ScriptInstance {
                        ast,
                        scope: Scope::new(),
                        started: false,
                        failed: false,
                        has_start,
                        has_update,
                    },
                );
            }
        }

        self.scripter.time += dt as f64;

        {
            let mut state = self.scripter.state.lock().unwrap();

            state.time = self.scripter.time;

            capture_keys(
                ctx,
                &mut state,
                self.mouse_locked && !ctx.wants_keyboard_input(),
            );
        }

        let count = self.scene.objects.len();

        for index in 0..count {
            let Some(mut instance) = self.scripter.instances.remove(&index) else {
                continue;
            };

            if instance.failed {
                self.scripter.instances.insert(index, instance);

                continue;
            }

            let player_position = self
                .scene
                .objects
                .iter()
                .find(|o| o.player_mode != 0)
                .map(|o| o.position)
                .unwrap_or([0.0; 3]);

            {
                let mut state = self.scripter.state.lock().unwrap();

                state.actions.clear();
                state.console.clear();

                state.position = self.scene.objects[index].position;

                state.rotation = self.scene.objects[index].rotation;

                state.player = player_position;
            }

            let result = (|| -> Result<(), String> {
                if !instance.started {
                    self.scripter
                        .engine
                        .run_ast_with_scope(&mut instance.scope, &instance.ast)
                        .map_err(|e| e.to_string())?;

                    if instance.has_start {
                        self.scripter
                            .engine
                            .call_fn_with_options::<()>(
                                CallFnOptions::new().eval_ast(false),
                                &mut instance.scope,
                                &instance.ast,
                                "start",
                                (),
                            )
                            .map_err(|e| e.to_string())?;
                    }

                    instance.started = true;
                }

                if instance.has_update {
                    self.scripter
                        .engine
                        .call_fn_with_options::<()>(
                            CallFnOptions::new().eval_ast(false),
                            &mut instance.scope,
                            &instance.ast,
                            "update",
                            (dt as f64,),
                        )
                        .map_err(|e| e.to_string())?;
                }

                Ok(())
            })();

            let (actions, messages) = {
                let mut state = self.scripter.state.lock().unwrap();

                (
                    std::mem::take(&mut state.actions),
                    std::mem::take(&mut state.console),
                )
            };

            let name = self.scene.objects[index].name.clone();

            for message in messages {
                self.scripter.log(format!("[{}] {}", name, message));
            }

            match result {
                Ok(()) => {
                    for action in actions {
                        self.apply_script_action(index, action);
                    }
                }

                Err(error) => {
                    self.scripter.log(format!("[ERROR] {}: {}", name, error));

                    instance.failed = true;
                }
            }

            self.scripter.instances.insert(index, instance);
        }
    }

    fn apply_script_action(&mut self, index: usize, action: Action) {
        let object = &mut self.scene.objects[index];

        let mut sync_transform = false;
        let mut velocity = None;
        let mut impulse = None;

        match action {
            Action::LightColor(v) => {
                object.light.color = v.map(|n| n.clamp(0.0, 1.0));
            }

            Action::Move(v) => {
                for i in 0..3 {
                    object.position[i] += v[i];
                }

                sync_transform = true;
            }

            Action::Rotate(v) => {
                for i in 0..3 {
                    object.rotation[i] += v[i];
                }

                sync_transform = true;
            }

            Action::Position(v) => {
                object.position = v;
                sync_transform = true;
            }

            Action::Rotation(v) => {
                object.rotation = v;
                sync_transform = true;
            }

            Action::Scale(v) => {
                object.scale = v.map(|n| n.max(0.05));
            }

            Action::Color(v) => {
                object.color = v.map(|n| n.clamp(0.0, 1.0));
            }

            Action::Light(v) => {
                object.light.energy = v.clamp(0.0, 10000.0);
            }

            Action::Emission(v) => {
                object.emission = v.max(0.0);
            }

            Action::Velocity(v) => {
                velocity = Some(v);
            }

            Action::Impulse(v) => {
                impulse = Some(v);
            }
        }

        if let Some(world) = &mut self.physics_world {
            let handle = if object.player_mode != 0 {
                world.player
            } else {
                world.handles.get(index).copied().flatten()
            };

            if let Some(handle) = handle {
                if let Some(body) = world.bodies.get_mut(handle) {
                    if sync_transform {
                        body.set_translation(
                            rapier3d::na::Vector3::new(
                                object.position[0],
                                object.position[1],
                                object.position[2],
                            ),
                            true,
                        );

                        body.set_rotation(
                            rapier3d::na::UnitQuaternion::from_euler_angles(
                                object.rotation[0].to_radians(),
                                object.rotation[1].to_radians(),
                                object.rotation[2].to_radians(),
                            ),
                            true,
                        );
                    }

                    if let Some(v) = velocity {
                        body.set_linvel(rapier3d::na::Vector3::new(v[0], v[1], v[2]), true);
                    }

                    if let Some(v) = impulse {
                        body.apply_impulse(rapier3d::na::Vector3::new(v[0], v[1], v[2]), true);
                    }
                }
            }
        }
    }
}

// ==========================================
// SYNTAX HIGHLIGHTING
// ==========================================

fn highlight(source: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();

    let chars: Vec<char> = source.chars().collect();

    let mut i = 0;

    let normal = egui::Color32::from_rgb(215, 220, 230);

    let keyword = egui::Color32::from_rgb(198, 160, 250);

    let function = egui::Color32::from_rgb(125, 190, 250);

    let string = egui::Color32::from_rgb(155, 215, 150);

    let comment = egui::Color32::from_rgb(110, 135, 120);

    let number = egui::Color32::from_rgb(245, 208, 145);

    while i < chars.len() {
        let start = i;
        let c = chars[i];

        let color;

        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }

            color = comment;
        } else if c == '"' {
            i += 1;

            while i < chars.len() {
                let ch = chars[i];
                i += 1;

                if ch == '\\' && i < chars.len() {
                    i += 1;
                    continue;
                }

                if ch == '"' {
                    break;
                }
            }

            color = string;
        } else if c.is_ascii_digit() {
            i += 1;

            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }

            color = number;
        } else if c.is_alphabetic() || c == '_' {
            i += 1;

            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }

            let word: String = chars[start..i].iter().collect();

            color = match word.as_str() {
                "func" | "fn" | "var" | "let" | "const" | "if" | "else" | "while" | "for"
                | "in" | "return" | "break" | "continue" | "true" | "false" => keyword,

                _ => {
                    if chars.get(i) == Some(&'(') {
                        function
                    } else {
                        normal
                    }
                }
            };
        } else {
            i += 1;
            color = normal;
        }

        let token: String = chars[start..i].iter().collect();

        job.append(
            &token,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),

                color,

                ..Default::default()
            },
        );
    }

    job
}

// ==========================================
// SCRIPT EDITOR
// ==========================================

impl GXScripter {
    fn show(&mut self, ctx: &egui::Context, project: &std::path::Path, playing: bool) {
        let mut open = self.open;

        egui::Window::new("GXScripter")
            .open(&mut open)
            .default_size([1150.0, 700.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("GXScript");

                    if ui.button("Explorer").clicked() {
                        self.explorer = !self.explorer;
                    }

                    if ui.button("Console").clicked() {
                        self.show_console = !self.show_console;
                    }

                    if ui.button("Save All").clicked() {
                        match self.save_pending(project) {
                            Ok(()) => {
                                self.log("[GXS] Scripts saved".into());
                            }

                            Err(error) => self.log(error),
                        }
                    }

                    if ui.button("Validate").clicked() {
                        if let Some(tab) = self.tabs.get(self.active) {
                            let result = self.engine.compile(translate(&tab.source));

                            match result {
                                Ok(_) => self.log("[GXS] Syntax valid".into()),

                                Err(error) => self.log(error.to_string()),
                            }
                        }
                    }

                    ui.label(if playing { "PLAYTEST" } else { "DEVELOPMENT" });
                });

                if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
                    if let Err(error) = self.save_pending(project) {
                        self.log(error);
                    }
                }

                ui.separator();

                let console_height = if self.show_console { 155.0 } else { 0.0 };

                let height = (ui.available_height() - console_height).max(220.0);

                ui.horizontal_top(|ui| {
                    if self.explorer {
                        ui.allocate_ui_with_layout(
                            egui::vec2(155.0, height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(155.0);

                                ui.strong("EXPLORER");
                                ui.separator();

                                ui.add(
                                    egui::TextEdit::singleline(&mut self.new_name)
                                        .desired_width(140.0),
                                );

                                if ui.button("+ New Script").clicked() {
                                    let name = self.new_name.clone();

                                    if let Err(error) = self.create_script(project, &name) {
                                        self.log(error);
                                    }
                                }

                                ui.separator();
                                ui.label("Scripts");

                                egui::ScrollArea::vertical()
                                    .id_salt("gx_files")
                                    .max_height(height - 100.0)
                                    .show(ui, |ui| {
                                        let directory = project.join("Assets/Scripts");

                                        if let Ok(entries) = fs::read_dir(directory) {
                                            let mut names: Vec<_> = entries
                                                .flatten()
                                                .map(|entry| {
                                                    entry.file_name().to_string_lossy().into_owned()
                                                })
                                                .filter(|name| valid_filename(name))
                                                .collect();

                                            names.sort();

                                            for name in names {
                                                let selected = self
                                                    .tabs
                                                    .get(self.active)
                                                    .map(|tab| tab.name == name)
                                                    .unwrap_or(false);

                                                if ui.selectable_label(selected, &name).clicked() {
                                                    if let Err(error) =
                                                        self.open_script(project, &name)
                                                    {
                                                        self.log(error);
                                                    }
                                                }
                                            }
                                        }
                                    });
                            },
                        );

                        ui.separator();
                    }

                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            let mut close = None;

                            for (i, tab) in self.tabs.iter().enumerate() {
                                let title = if tab.dirty {
                                    format!("{} *", tab.name)
                                } else {
                                    tab.name.clone()
                                };

                                if ui.selectable_label(self.active == i, title).clicked() {
                                    self.active = i;
                                }

                                if ui.small_button("×").clicked() {
                                    if !tab.dirty {
                                        close = Some(i);
                                    }
                                }

                                ui.separator();
                            }

                            if let Some(index) = close {
                                self.tabs.remove(index);

                                self.active = self.active.min(self.tabs.len().saturating_sub(1));
                            }
                        });

                        ui.separator();

                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            ui.label(format!("Assets / Scripts / {}", tab.name));

                            let editor_height = (height - 70.0).max(150.0);

                            let mut layouter = |ui: &egui::Ui, text: &str, width: f32| {
                                let mut job = highlight(text);

                                job.wrap.max_width = width;

                                ui.fonts(|fonts| fonts.layout_job(job))
                            };

                            egui::ScrollArea::both()
                                .id_salt("gx_code")
                                .max_height(editor_height)
                                .show(ui, |ui| {
                                    let width = ui.available_width();

                                    ui.horizontal_top(|ui| {
                                        let numbers = (1..=tab.source.lines().count().max(1))
                                            .map(|n| n.to_string())
                                            .collect::<Vec<_>>()
                                            .join("\n");

                                        ui.label(
                                            egui::RichText::new(numbers)
                                                .monospace()
                                                .size(14.0)
                                                .color(egui::Color32::from_gray(110)),
                                        );

                                        let response = ui.add(
                                            egui::TextEdit::multiline(&mut tab.source)
                                                .code_editor()
                                                .font(egui::FontId::monospace(14.0))
                                                .layouter(&mut layouter)
                                                .desired_width((width - 50.0).max(150.0))
                                                .desired_rows((editor_height / 18.0) as usize),
                                        );

                                        if response.changed() {
                                            tab.dirty = true;
                                        }
                                    });
                                });

                            ui.small(format!(
                                "GXScript | UTF-8 | {} lines",
                                tab.source.lines().count()
                            ));
                        } else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(60.0);
                                ui.heading("GXScripter");
                                ui.label("Create or open a script.");
                            });
                        }
                    });
                });

                if self.show_console {
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.strong("OUTPUT");

                        if ui.button("Clear").clicked() {
                            self.console.clear();
                        }
                    });

                    egui::ScrollArea::vertical()
                        .id_salt("gx_console")
                        .max_height(115.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for line in &self.console {
                                let color = if line.contains("ERROR") {
                                    egui::Color32::LIGHT_RED
                                } else {
                                    egui::Color32::LIGHT_GRAY
                                };

                                ui.label(egui::RichText::new(line).monospace().color(color));
                            }
                        });
                }
            });

        self.open = open;
    }
}

// ==========================================
// OBJECT SCRIPT INSPECTOR
// ==========================================

impl GXEngine {
    pub(super) fn script_inspector(&mut self, ui: &mut egui::Ui, index: usize) {
        ui.separator();
        ui.heading("GXScript");

        let mut selected = self.scene.objects[index].script.clone();

        egui::ComboBox::from_id_salt(("object_script", index))
            .selected_text(if selected.is_empty() {
                "No script"
            } else {
                &selected
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, String::new(), "None");

                let directory = self.project_dir.join("Assets/Scripts");

                if let Ok(entries) = fs::read_dir(directory) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();

                        if valid_filename(&name) {
                            ui.selectable_value(&mut selected, name.clone(), name);
                        }
                    }
                }
            });

        if !self.playing {
            self.scene.objects[index].script = selected.clone();

            ui.horizontal(|ui| {
                if ui.button("Create & Attach").clicked() {
                    let name = format!("object_{}", index);

                    match self.scripter.create_script(&self.project_dir, &name) {
                        Ok(script) => {
                            self.scene.objects[index].script = script;
                        }

                        Err(error) => {
                            self.scripter.log(error);
                        }
                    }
                }

                if !selected.is_empty() {
                    if ui.button("Edit Script").clicked() {
                        if let Err(error) = self.scripter.open_script(&self.project_dir, &selected)
                        {
                            self.scripter.log(error);
                        }
                    }

                    if ui.button("Detach").clicked() {
                        self.scene.objects[index].script.clear();
                    }
                }
            });
        }
    }

    pub(super) fn scripting_window(&mut self, ctx: &egui::Context) {
        if self.scripter.open {
            self.scripter.show(ctx, &self.project_dir, self.playing);
        }
    }
}

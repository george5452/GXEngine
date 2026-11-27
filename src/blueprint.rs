use super::*;
use rhai::{Dynamic, Engine, Scope};
use std::collections::HashMap;

// ==========================================
// GXBLUEPRINT DATA
// ==========================================

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) enum NodeKind {
    Start,
    Update,
    KeyPressed,
    KeyHeld,
    SetVariable,
    Branch,
    Script,
    Move,
    Rotate,
    SetPosition,
    SetRotation,
    SetScale,
    SetColor,
    SetMetallic,
    SetRoughness,
    SetEmission,
    SetVelocity,
    AddImpulse,
    Teleport,
    Print,
    Call,
    Return,
    Delay,
    Stop,
}

impl NodeKind {
    fn all() -> &'static [Self] {
        &[
            Self::Start,
            Self::Update,
            Self::KeyPressed,
            Self::KeyHeld,
            Self::SetVariable,
            Self::Branch,
            Self::Script,
            Self::Move,
            Self::Rotate,
            Self::SetPosition,
            Self::SetRotation,
            Self::SetScale,
            Self::SetColor,
            Self::SetMetallic,
            Self::SetRoughness,
            Self::SetEmission,
            Self::SetVelocity,
            Self::AddImpulse,
            Self::Teleport,
            Self::Print,
            Self::Call,
            Self::Return,
            Self::Delay,
            Self::Stop,
        ]
    }

    fn is_event(self) -> bool {
        matches!(
            self,
            Self::Start | Self::Update | Self::KeyPressed | Self::KeyHeld
        )
    }

    fn color(self) -> egui::Color32 {
        if self.is_event() {
            egui::Color32::from_rgb(170, 60, 75)
        } else {
            match self {
                Self::Branch | Self::Call | Self::Return | Self::Delay => {
                    egui::Color32::from_rgb(120, 75, 180)
                }

                Self::SetVariable | Self::Script => egui::Color32::from_rgb(50, 140, 100),

                _ => egui::Color32::from_rgb(60, 100, 175),
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct BlueprintNode {
    id: u32,
    kind: NodeKind,
    title: String,

    target: String,
    expression: String,
    variable: String,

    next: Option<u32>,
    alternate: Option<u32>,

    position: [f32; 2],
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Blueprint {
    enabled: bool,
    nodes: Vec<BlueprintNode>,
    next_id: u32,
}

impl Default for Blueprint {
    fn default() -> Self {
        Self {
            enabled: true,
            nodes: Vec::new(),
            next_id: 1,
        }
    }
}

impl Blueprint {
    fn add_node(&mut self, kind: NodeKind) {
        let id = self.next_id;
        self.next_id += 1;

        let expression = match kind {
            NodeKind::Branch => "true",
            NodeKind::SetVariable => "100",
            NodeKind::KeyPressed | NodeKind::KeyHeld => "E",
            NodeKind::Delay => "1.0",
            NodeKind::Print => "\"Hello GXEngine\"",
            NodeKind::Script => "let value = 1;",
            NodeKind::SetScale => "[1.0, 1.0, 1.0]",
            NodeKind::SetColor => "[1.0, 1.0, 1.0]",
            NodeKind::SetMetallic => "0.5",
            NodeKind::SetRoughness => "0.5",
            NodeKind::SetEmission => "1.0",
            _ => "[0.0, 0.0, 0.0]",
        };

        self.nodes.push(BlueprintNode {
            id,
            kind,
            title: format!("{:?}", kind),
            target: "self".into(),
            expression: expression.into(),
            variable: "value".into(),
            next: None,
            alternate: None,
            position: [
                30.0 + ((id - 1) % 3) as f32 * 200.0,
                30.0 + ((id - 1) / 3) as f32 * 110.0,
            ],
        });
    }
}

// ==========================================
// RUNTIME DATA
// ==========================================

#[derive(Clone)]
struct Task {
    node: u32,
    wake_time: f32,
    stack: Vec<Option<u32>>,
}

#[derive(Default)]
struct Instance {
    scope: Scope<'static>,
    tasks: Vec<Task>,
    started: bool,
}

pub(super) struct BlueprintRuntime {
    engine: Engine,
    instances: HashMap<usize, Instance>,

    time: f32,
    was_playing: bool,

    pub open: bool,
    selected_node: Option<u32>,
    selected_object: Option<usize>,
    pan: egui::Vec2,

    console: Vec<String>,
}

impl Default for BlueprintRuntime {
    fn default() -> Self {
        let mut engine = Engine::new();

        engine.set_max_operations(20_000);
        engine.set_max_call_levels(32);
        engine.set_max_variables(256);

        engine.register_fn("distance", |a: rhai::Array, b: rhai::Array| -> f64 {
            match (
                dynamic_vector(Dynamic::from(a)),
                dynamic_vector(Dynamic::from(b)),
            ) {
                (Ok(a), Ok(b)) => Vec3::from_array(a).distance(Vec3::from_array(b)) as f64,
                _ => 0.0,
            }
        });

        Self {
            engine,
            instances: HashMap::new(),
            time: 0.0,
            was_playing: false,
            open: false,
            selected_node: None,
            selected_object: None,
            pan: egui::Vec2::ZERO,
            console: Vec::new(),
        }
    }
}

impl BlueprintRuntime {
    fn log(&mut self, message: String) {
        self.console.push(message);

        if self.console.len() > 200 {
            self.console.remove(0);
        }
    }

    fn reset(&mut self) {
        self.instances.clear();
        self.time = 0.0;
    }
}

// ==========================================
// EXPRESSIONS
// ==========================================

fn dynamic_number(value: Dynamic) -> Result<f32, String> {
    if value.is::<rhai::FLOAT>() {
        Ok(value.cast::<rhai::FLOAT>() as f32)
    } else if value.is::<rhai::INT>() {
        Ok(value.cast::<rhai::INT>() as f32)
    } else {
        Err("Expected number".into())
    }
}

fn dynamic_vector(value: Dynamic) -> Result<[f32; 3], String> {
    let array = value
        .try_cast::<rhai::Array>()
        .ok_or("Expected [x, y, z]")?;

    if array.len() != 3 {
        return Err("Vector must contain 3 numbers".into());
    }

    let result = [
        dynamic_number(array[0].clone())?,
        dynamic_number(array[1].clone())?,
        dynamic_number(array[2].clone())?,
    ];

    if result.iter().any(|v| !v.is_finite()) {
        return Err("Invalid vector".into());
    }

    Ok(result)
}

fn array_value(value: [f32; 3]) -> rhai::Array {
    value
        .into_iter()
        .map(|v| Dynamic::from(v as rhai::FLOAT))
        .collect()
}

fn parse_key(name: &str) -> Option<egui::Key> {
    use egui::Key::*;

    Some(match name.trim().to_uppercase().as_str() {
        "A" => A,
        "B" => B,
        "C" => C,
        "D" => D,
        "E" => E,
        "F" => F,
        "G" => G,
        "H" => H,
        "I" => I,
        "J" => J,
        "K" => K,
        "L" => L,
        "M" => M,
        "N" => N,
        "O" => O,
        "P" => P,
        "Q" => Q,
        "R" => R,
        "S" => S,
        "T" => T,
        "U" => U,
        "V" => V,
        "W" => W,
        "X" => X,
        "Y" => Y,
        "Z" => Z,
        "SPACE" => Space,
        "ENTER" => Enter,
        "ESCAPE" => Escape,
        "UP" => ArrowUp,
        "DOWN" => ArrowDown,
        "LEFT" => ArrowLeft,
        "RIGHT" => ArrowRight,
        _ => return None,
    })
}

fn target_index(scene: &Scene, owner: usize, target: &str) -> Option<usize> {
    match target.trim() {
        "" | "self" => Some(owner),

        "player" => scene.objects.iter().position(|o| o.player_mode != 0),

        name => scene.objects.iter().position(|o| o.name == name),
    }
}

// ==========================================
// BLUEPRINT EXECUTION
// ==========================================

impl GXEngine {
    pub(super) fn blueprint_update(&mut self, ctx: &egui::Context, dt: f32) {
        let mut runtime = std::mem::take(&mut self.blueprint_runtime);

        if !self.playing {
            if runtime.was_playing {
                runtime.reset();
            }

            runtime.was_playing = false;
            self.blueprint_runtime = runtime;
            return;
        }

        if !runtime.was_playing {
            runtime.reset();
            runtime.was_playing = true;
        }

        runtime.time += dt;

        let count = self.scene.objects.len();

        for owner in 0..count {
            let graph = self.scene.objects[owner].blueprint.clone();

            if !graph.enabled || graph.nodes.is_empty() {
                continue;
            }

            let mut instance = runtime.instances.remove(&owner).unwrap_or_default();

            instance.scope.set_value("dt", dt as f64);

            instance.scope.set_value("time", runtime.time as f64);

            instance
                .scope
                .set_value("position", array_value(self.scene.objects[owner].position));

            instance
                .scope
                .set_value("rotation", array_value(self.scene.objects[owner].rotation));

            instance
                .scope
                .set_value("scale", array_value(self.scene.objects[owner].scale));

            let player_position = self
                .scene
                .objects
                .iter()
                .find(|o| o.player_mode != 0)
                .map(|o| o.position)
                .unwrap_or([0.0; 3]);

            instance
                .scope
                .set_value("player_position", array_value(player_position));

            let mut tasks = std::mem::take(&mut instance.tasks);

            for node in &graph.nodes {
                let triggered = match node.kind {
                    NodeKind::Start => !instance.started,

                    NodeKind::Update => true,

                    NodeKind::KeyPressed => parse_key(&node.expression)
                        .map(|key| self.mouse_locked && ctx.input(|i| i.key_pressed(key)))
                        .unwrap_or(false),

                    NodeKind::KeyHeld => parse_key(&node.expression)
                        .map(|key| self.mouse_locked && ctx.input(|i| i.key_down(key)))
                        .unwrap_or(false),

                    _ => false,
                };

                if triggered {
                    if let Some(next) = node.next {
                        if tasks.len() < 64 {
                            tasks.push(Task {
                                node: next,
                                wake_time: runtime.time,
                                stack: Vec::new(),
                            });
                        }
                    }
                }
            }

            instance.started = true;

            // Maximum graph operations per object per frame.
            let mut budget = 256usize;

            for mut task in tasks {
                if task.wake_time > runtime.time {
                    instance.tasks.push(task);
                    continue;
                }

                loop {
                    if budget == 0 {
                        runtime.log("Blueprint execution limit reached".into());
                        break;
                    }

                    budget -= 1;

                    let Some(node) = graph.nodes.iter().find(|n| n.id == task.node) else {
                        break;
                    };

                    let mut next = node.next;

                    let result: Result<(), String> = (|| {
                        match node.kind {
                            NodeKind::Branch => {
                                let condition = runtime
                                    .engine
                                    .eval_with_scope::<bool>(&mut instance.scope, &node.expression)
                                    .map_err(|e| e.to_string())?;

                                next = if condition { node.next } else { node.alternate };
                            }

                            NodeKind::SetVariable => {
                                let value = runtime
                                    .engine
                                    .eval_with_scope::<Dynamic>(
                                        &mut instance.scope,
                                        &node.expression,
                                    )
                                    .map_err(|e| e.to_string())?;

                                instance.scope.set_value(node.variable.clone(), value);
                            }

                            NodeKind::Script => {
                                runtime
                                    .engine
                                    .run_with_scope(&mut instance.scope, &node.expression)
                                    .map_err(|e| e.to_string())?;
                            }

                            NodeKind::Print => {
                                let value = runtime
                                    .engine
                                    .eval_with_scope::<Dynamic>(
                                        &mut instance.scope,
                                        &node.expression,
                                    )
                                    .map_err(|e| e.to_string())?;

                                runtime
                                    .log(format!("{}: {}", self.scene.objects[owner].name, value,));
                            }

                            NodeKind::Call => {
                                if task.stack.len() >= 32 {
                                    return Err("Function depth limit".into());
                                }

                                task.stack.push(node.next);
                                next = node.alternate;
                            }

                            NodeKind::Return => {
                                next = task.stack.pop().flatten();
                            }

                            NodeKind::Delay => {
                                let value = runtime
                                    .engine
                                    .eval_with_scope::<Dynamic>(
                                        &mut instance.scope,
                                        &node.expression,
                                    )
                                    .map_err(|e| e.to_string())?;

                                let seconds = dynamic_number(value)?;

                                if let Some(id) = next {
                                    instance.tasks.push(Task {
                                        node: id,
                                        wake_time: runtime.time + seconds.clamp(0.0, 3600.0),
                                        stack: task.stack.clone(),
                                    });
                                }

                                next = None;
                            }

                            NodeKind::Stop => {
                                next = None;
                            }

                            kind if kind.is_event() => {}

                            _ => {
                                let value = runtime
                                    .engine
                                    .eval_with_scope::<Dynamic>(
                                        &mut instance.scope,
                                        &node.expression,
                                    )
                                    .map_err(|e| e.to_string())?;

                                self.blueprint_action(owner, node, value)?;
                            }
                        }

                        Ok(())
                    })();

                    if let Err(error) = result {
                        runtime.log(format!("Node {}: {}", node.id, error,));

                        break;
                    }

                    match next {
                        Some(id) => task.node = id,

                        None => {
                            if let Some(Some(id)) = task.stack.pop() {
                                task.node = id;
                            } else {
                                break;
                            }
                        }
                    }
                }
            }

            runtime.instances.insert(owner, instance);
        }

        self.blueprint_runtime = runtime;
    }

    fn blueprint_action(
        &mut self,
        owner: usize,
        node: &BlueprintNode,
        value: Dynamic,
    ) -> Result<(), String> {
        let index =
            target_index(&self.scene, owner, &node.target).ok_or("Target object not found")?;

        let object = &mut self.scene.objects[index];

        match node.kind {
            NodeKind::Move => {
                let v = dynamic_vector(value)?;

                for i in 0..3 {
                    object.position[i] += v[i];
                }
            }

            NodeKind::Rotate => {
                let v = dynamic_vector(value)?;

                for i in 0..3 {
                    object.rotation[i] += v[i];
                }
            }

            NodeKind::SetPosition | NodeKind::Teleport => {
                object.position = dynamic_vector(value)?;
            }

            NodeKind::SetRotation => {
                object.rotation = dynamic_vector(value)?;
            }

            NodeKind::SetScale => {
                object.scale = dynamic_vector(value)?.map(|v| v.max(0.05));
            }

            NodeKind::SetColor => {
                object.color = dynamic_vector(value)?.map(|v| v.clamp(0.0, 1.0));
            }

            NodeKind::SetMetallic => {
                object.metallic = dynamic_number(value)?.clamp(0.0, 1.0);
            }

            NodeKind::SetRoughness => {
                object.roughness = dynamic_number(value)?.clamp(0.02, 1.0);
            }

            NodeKind::SetEmission => {
                object.emission = dynamic_number(value)?.max(0.0);
            }

            NodeKind::SetVelocity | NodeKind::AddImpulse => {
                let v = dynamic_vector(value)?;

                if let Some(world) = &mut self.physics_world {
                    let handle = if object.player_mode != 0 {
                        world.player
                    } else {
                        world.handles.get(index).copied().flatten()
                    };

                    if let Some(handle) = handle {
                        if let Some(body) = world.bodies.get_mut(handle) {
                            let vector = rapier3d::na::Vector3::new(v[0], v[1], v[2]);

                            if node.kind == NodeKind::SetVelocity {
                                body.set_linvel(vector, true);
                            } else {
                                body.apply_impulse(vector, true);
                            }
                        }
                    }
                }
            }

            _ => {}
        }

        // Keep physics and visual transforms synchronized.
        if matches!(
            node.kind,
            NodeKind::Move
                | NodeKind::Rotate
                | NodeKind::SetPosition
                | NodeKind::SetRotation
                | NodeKind::Teleport
        ) {
            if let Some(world) = &mut self.physics_world {
                let handle = if object.player_mode != 0 {
                    world.player
                } else {
                    world.handles.get(index).copied().flatten()
                };

                if let Some(handle) = handle {
                    if let Some(body) = world.bodies.get_mut(handle) {
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
                }
            }
        }

        Ok(())
    }
}

// ==========================================
// BLUEPRINT EDITOR
// ==========================================

fn node_picker(ui: &mut egui::Ui, label: &str, value: &mut Option<u32>, choices: &[(u32, String)]) {
    let selected = choices
        .iter()
        .find(|(id, _)| Some(*id) == *value)
        .map(|(_, name)| name.as_str())
        .unwrap_or("None");

    egui::ComboBox::from_id_salt(label)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, "None");

            for (id, name) in choices {
                ui.selectable_value(value, Some(*id), name);
            }
        });
}

impl GXEngine {
    pub(super) fn blueprint_window(&mut self, ctx: &egui::Context) {
        if !self.blueprint_runtime.open {
            return;
        }

        let mut open = true;

        egui::Window::new("GXBlueprint")
            .open(&mut open)
            .default_size([1050.0, 650.0])
            .resizable(true)
            .show(ctx, |ui| {
                if self.scene.objects.is_empty() {
                    ui.label("Create an object first.");
                    return;
                }

                let runtime = &mut self.blueprint_runtime;

                let mut owner = runtime
                    .selected_object
                    .or(self.selected)
                    .unwrap_or(0)
                    .min(self.scene.objects.len() - 1);

                ui.horizontal(|ui| {
                    ui.heading("GXBlueprint");

                    egui::ComboBox::from_id_salt("bp_object")
                        .selected_text(&self.scene.objects[owner].name)
                        .show_ui(ui, |ui| {
                            for (i, object) in self.scene.objects.iter().enumerate() {
                                ui.selectable_value(&mut owner, i, &object.name);
                            }
                        });
                });

                if runtime.selected_object != Some(owner) {
                    runtime.selected_object = Some(owner);
                    runtime.selected_node = None;
                    runtime.pan = egui::Vec2::ZERO;
                }

                let graph = &mut self.scene.objects[owner].blueprint;

                ui.add_enabled_ui(!self.playing, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut graph.enabled, "Enabled");

                        ui.menu_button("+ Add Node", |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(400.0)
                                .show(ui, |ui| {
                                    for &kind in NodeKind::all() {
                                        if ui.button(format!("{:?}", kind)).clicked() {
                                            graph.add_node(kind);
                                            ui.close_menu();
                                        }
                                    }
                                });
                        });

                        if ui.button("Delete Node").clicked() {
                            if let Some(id) = runtime.selected_node.take() {
                                graph.nodes.retain(|n| n.id != id);

                                for node in &mut graph.nodes {
                                    if node.next == Some(id) {
                                        node.next = None;
                                    }

                                    if node.alternate == Some(id) {
                                        node.alternate = None;
                                    }
                                }
                            }
                        }

                        if ui.button("Reset View").clicked() {
                            runtime.pan = egui::Vec2::ZERO;
                        }
                    });
                });

                ui.separator();

                ui.columns(2, |columns| {
                    let ui = &mut columns[0];

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 420.0),
                        egui::Sense::drag(),
                    );

                    if response.dragged_by(egui::PointerButton::Middle) {
                        runtime.pan += ui.input(|i| i.pointer.delta());
                    }

                    let painter = ui.painter().with_clip_rect(rect);

                    painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(22, 24, 30));

                    let origin = rect.min + runtime.pan;

                    for x in 0..40 {
                        let x = rect.left() + x as f32 * 24.0;

                        painter.line_segment(
                            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                            egui::Stroke::new(0.5_f32, egui::Color32::from_gray(45)),
                        );
                    }

                    for y in 0..20 {
                        let y = rect.top() + y as f32 * 24.0;

                        painter.line_segment(
                            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                            egui::Stroke::new(0.5_f32, egui::Color32::from_gray(45)),
                        );
                    }

                    // Draw execution connections.
                    for node in &graph.nodes {
                        let start =
                            origin + egui::vec2(node.position[0] + 170.0, node.position[1] + 38.0);

                        for (target, color) in [
                            (node.next, egui::Color32::WHITE),
                            (node.alternate, egui::Color32::LIGHT_RED),
                        ] {
                            if let Some(target) = target {
                                if let Some(other) = graph.nodes.iter().find(|n| n.id == target) {
                                    let end = origin
                                        + egui::vec2(other.position[0], other.position[1] + 38.0);

                                    painter.line_segment(
                                        [start, end],
                                        egui::Stroke::new(2.0_f32, color),
                                    );
                                }
                            }
                        }
                    }

                    // Draw draggable nodes.
                    for node in &mut graph.nodes {
                        let pos = origin + egui::vec2(node.position[0], node.position[1]);

                        let node_rect = egui::Rect::from_min_size(pos, egui::vec2(170.0, 76.0));

                        if !rect.intersects(node_rect) {
                            continue;
                        }

                        let interaction = ui.interact(
                            node_rect.intersect(rect),
                            ui.id().with(node.id),
                            egui::Sense::click_and_drag(),
                        );

                        if interaction.clicked() || interaction.dragged() {
                            runtime.selected_node = Some(node.id);
                        }

                        if interaction.dragged() && !self.playing {
                            let delta = ui.input(|i| i.pointer.delta());

                            node.position[0] += delta.x;
                            node.position[1] += delta.y;
                        }

                        painter.rect_filled(node_rect, 5.0, egui::Color32::from_rgb(42, 45, 55));

                        let header = egui::Rect::from_min_size(pos, egui::vec2(170.0, 26.0));

                        painter.rect_filled(header, 5.0, node.kind.color());

                        painter.text(
                            header.center(),
                            egui::Align2::CENTER_CENTER,
                            &node.title,
                            egui::FontId::proportional(13.0),
                            egui::Color32::WHITE,
                        );

                        painter.text(
                            pos + egui::vec2(10.0, 42.0),
                            egui::Align2::LEFT_CENTER,
                            format!("#{} | {:?}", node.id, node.kind,),
                            egui::FontId::proportional(12.0),
                            egui::Color32::LIGHT_GRAY,
                        );

                        let color = if runtime.selected_node == Some(node.id) {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::WHITE
                        };

                        painter.circle_filled(pos + egui::vec2(0.0, 38.0), 4.0, color);

                        painter.circle_filled(pos + egui::vec2(170.0, 38.0), 4.0, color);
                    }

                    ui.label("Drag nodes to move them.");
                    ui.label("Middle mouse to pan.");

                    // Node inspector.
                    let ui = &mut columns[1];

                    ui.heading("Node Inspector");

                    let choices: Vec<(u32, String)> = graph
                        .nodes
                        .iter()
                        .map(|n| (n.id, format!("#{} {}", n.id, n.title)))
                        .collect();

                    ui.add_enabled_ui(!self.playing, |ui| {
                        if let Some(node) = graph
                            .nodes
                            .iter_mut()
                            .find(|n| Some(n.id) == runtime.selected_node)
                        {
                            ui.label(format!("Node #{}", node.id));

                            ui.text_edit_singleline(&mut node.title);

                            ui.label(format!("{:?}", node.kind));

                            ui.separator();

                            ui.label("Target object");

                            ui.text_edit_singleline(&mut node.target);

                            ui.small("self, player, or object name");

                            ui.label("Variable name");

                            ui.text_edit_singleline(&mut node.variable);

                            ui.label("Expression / Script / Key");

                            ui.add(
                                egui::TextEdit::multiline(&mut node.expression)
                                    .code_editor()
                                    .desired_rows(6),
                            );

                            ui.separator();

                            ui.label("Next / True");

                            node_picker(ui, "bp_next", &mut node.next, &choices);

                            if matches!(node.kind, NodeKind::Branch | NodeKind::Call) {
                                ui.label("False / Function entry");

                                node_picker(ui, "bp_alt", &mut node.alternate, &choices);
                            }
                        } else {
                            ui.label("Select a node.");
                        }
                    });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.heading("Blueprint Console");

                    if ui.button("Clear").clicked() {
                        runtime.console.clear();
                    }
                });

                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &runtime.console {
                            ui.monospace(line);
                        }
                    });
            });

        self.blueprint_runtime.open = open;
    }
}

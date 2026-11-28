use super::*;

// ==========================================
// GXUI DATA
// ==========================================

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum WidgetKind {
    Panel,
    Label,
    Button,
    ProgressBar,
    Checkbox,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum ButtonAction {
    None,
    LoadScene,
    ToggleVisibility,
    Quit,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Widget {
    script: String,
    id: u64,
    name: String,
    kind: WidgetKind,

    position: [f32; 2],
    size: [f32; 2],
    anchor: usize,

    color: [f32; 4],
    text_color: [f32; 4],

    text: String,
    font_size: f32,
    radius: f32,

    visible: bool,
    value: f32,

    action: ButtonAction,
    target: String,
}

impl Default for Widget {
    fn default() -> Self {
        Self {
            script: String::new(),
            id: 0,
            name: "Widget".into(),
            kind: WidgetKind::Panel,

            position: [0.0, 0.0],
            size: [240.0, 70.0],
            anchor: 4,

            color: [0.18, 0.20, 0.24, 1.0],
            text_color: [1.0; 4],

            text: "Text".into(),
            font_size: 28.0,
            radius: 8.0,

            visible: true,
            value: 0.75,

            action: ButtonAction::None,
            target: String::new(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Canvas {
    widgets: Vec<Widget>,
    next_id: u64,

    pub ui_only: bool,

    resolution: [f32; 2],
    background: [f32; 4],
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            widgets: Vec::new(),
            next_id: 1,

            ui_only: false,

            resolution: [1920.0, 1080.0],
            background: [0.07, 0.08, 0.1, 1.0],
        }
    }
}

impl Canvas {
    fn add(&mut self, kind: WidgetKind) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let mut widget = Widget {
            id,
            kind,
            name: format!("{:?} {}", kind, id),
            ..Default::default()
        };

        match kind {
            WidgetKind::Panel => {
                widget.size = [500.0, 350.0];
                widget.text.clear();
            }

            WidgetKind::Label => {
                widget.size = [400.0, 80.0];
                widget.text = "New Label".into();
                widget.color[3] = 0.0;
                widget.font_size = 42.0;
            }

            WidgetKind::Button => {
                widget.text = "Button".into();
                widget.color = [0.18, 0.32, 0.52, 1.0];
            }

            WidgetKind::ProgressBar => {
                widget.size = [300.0, 30.0];
                widget.color = [0.20, 0.75, 0.35, 1.0];
            }

            WidgetKind::Checkbox => {
                widget.text = "Enabled".into();
                widget.size = [280.0, 50.0];
                widget.value = 0.0;
            }
        }

        self.widgets.push(widget);

        id
    }
}

// ==========================================
// EDITOR STATE
// ==========================================

#[derive(Default)]
pub(super) struct GUIEditor {
    pub open: bool,
    selected: Option<u64>,
}

// ==========================================
// LAYOUT
// ==========================================

fn color(c: [f32; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
        (c[3].clamp(0.0, 1.0) * 255.0) as u8,
    )
}

fn canvas_rect(available: egui::Rect, resolution: [f32; 2]) -> (egui::Rect, f32) {
    let scale = (available.width() / resolution[0].max(1.0))
        .min(available.height() / resolution[1].max(1.0))
        .max(0.0001);

    let size = egui::vec2(resolution[0] * scale, resolution[1] * scale);

    (
        egui::Rect::from_center_size(available.center(), size),
        scale,
    )
}

fn widget_rect(widget: &Widget, canvas: egui::Rect, scale: f32) -> egui::Rect {
    let anchor = widget.anchor.min(8);

    let ax = (anchor % 3) as f32 * 0.5;
    let ay = (anchor / 3) as f32 * 0.5;

    let size = egui::vec2(widget.size[0].max(1.0), widget.size[1].max(1.0)) * scale;

    let origin = canvas.min + egui::vec2(canvas.width() * ax, canvas.height() * ay);

    let position = origin + egui::vec2(widget.position[0], widget.position[1]) * scale
        - egui::vec2(size.x * ax, size.y * ay);

    egui::Rect::from_min_size(position, size)
}

// ==========================================
// SHARED UI DRAWING
// ==========================================

fn draw_widget(
    painter: &egui::Painter,
    widget: &Widget,
    rect: egui::Rect,
    scale: f32,
    hovered: bool,
) {
    let mut fill = color(widget.color);

    if hovered && widget.kind == WidgetKind::Button {
        fill = fill.gamma_multiply(1.2);
    }

    let radius = (widget.radius * scale).clamp(0.0, 100.0) as u8;

    let text_color = color(widget.text_color);

    let font = egui::FontId::proportional((widget.font_size * scale).max(1.0));

    match widget.kind {
        WidgetKind::Panel => {
            painter.rect_filled(rect, radius, fill);
        }

        WidgetKind::Label => {
            painter.rect_filled(rect, radius, fill);

            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &widget.text,
                font,
                text_color,
            );
        }

        WidgetKind::Button => {
            painter.rect_filled(rect, radius, fill);

            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &widget.text,
                font,
                text_color,
            );
        }

        WidgetKind::ProgressBar => {
            painter.rect_filled(rect, radius, egui::Color32::from_gray(35));

            let width = rect.width() * widget.value.clamp(0.0, 1.0);

            if width > 0.0 {
                painter.rect_filled(
                    egui::Rect::from_min_size(rect.min, egui::vec2(width, rect.height())),
                    radius,
                    fill,
                );
            }
        }

        WidgetKind::Checkbox => {
            let box_size = rect.height().min(30.0 * scale);

            let box_rect = egui::Rect::from_center_size(
                egui::pos2(rect.left() + box_size * 0.5, rect.center().y),
                egui::vec2(box_size, box_size),
            );

            painter.rect_filled(box_rect, radius, fill);

            if widget.value > 0.5 {
                painter.line_segment(
                    [
                        box_rect.left_center() + egui::vec2(box_size * 0.15, 0.0),
                        box_rect.center_bottom() - egui::vec2(0.0, box_size * 0.2),
                    ],
                    egui::Stroke::new(2.0_f32, text_color),
                );

                painter.line_segment(
                    [
                        box_rect.center_bottom() - egui::vec2(0.0, box_size * 0.2),
                        box_rect.right_top() + egui::vec2(-box_size * 0.15, box_size * 0.2),
                    ],
                    egui::Stroke::new(2.0_f32, text_color),
                );
            }

            painter.text(
                egui::pos2(box_rect.right() + 10.0 * scale, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &widget.text,
                font,
                text_color,
            );
        }
    }
}

// ==========================================
// UI WORKSPACE
// ==========================================

impl GXEngine {
    pub(super) fn gui_workspace(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("gxui_hierarchy")
            .default_width(190.0)
            .show(ctx, |ui| {
                ui.heading("UI Hierarchy");
                ui.separator();

                ui.menu_button("+ Add UI Element", |ui| {
                    for kind in [
                        WidgetKind::Panel,
                        WidgetKind::Label,
                        WidgetKind::Button,
                        WidgetKind::ProgressBar,
                        WidgetKind::Checkbox,
                    ] {
                        if ui.button(format!("{:?}", kind)).clicked() {
                            let id = self.scene.canvas.add(kind);

                            self.gui_editor.selected = Some(id);

                            ui.close_menu();
                        }
                    }
                });

                ui.separator();

                if ui
                    .selectable_label(self.gui_editor.selected.is_none(), "Canvas")
                    .clicked()
                {
                    self.gui_editor.selected = None;
                }

                for widget in &self.scene.canvas.widgets {
                    if ui
                        .selectable_label(self.gui_editor.selected == Some(widget.id), &widget.name)
                        .clicked()
                    {
                        self.gui_editor.selected = Some(widget.id);
                    }
                }

                ui.separator();

                if ui.button("Duplicate").clicked() {
                    if let Some(widget) = self
                        .scene
                        .canvas
                        .widgets
                        .iter()
                        .find(|w| Some(w.id) == self.gui_editor.selected)
                        .cloned()
                    {
                        let mut widget = widget;

                        widget.id = self.scene.canvas.next_id;

                        self.scene.canvas.next_id += 1;

                        widget.position[0] += 20.0;
                        widget.position[1] += 20.0;

                        widget.name.push_str(" Copy");

                        self.gui_editor.selected = Some(widget.id);

                        self.scene.canvas.widgets.push(widget);
                    }
                }

                if ui.button("Delete").clicked() {
                    if let Some(id) = self.gui_editor.selected.take() {
                        self.scene.canvas.widgets.retain(|w| w.id != id);
                    }
                }

                if ui.button("Bring to Front").clicked() {
                    if let Some(index) = self
                        .scene
                        .canvas
                        .widgets
                        .iter()
                        .position(|w| Some(w.id) == self.gui_editor.selected)
                    {
                        let widget = self.scene.canvas.widgets.remove(index);

                        self.scene.canvas.widgets.push(widget);
                    }
                }

                if ui.button("Send to Back").clicked() {
                    if let Some(index) = self
                        .scene
                        .canvas
                        .widgets
                        .iter()
                        .position(|w| Some(w.id) == self.gui_editor.selected)
                    {
                        let widget = self.scene.canvas.widgets.remove(index);

                        self.scene.canvas.widgets.insert(0, widget);
                    }
                }
            });

        // Find scene files for button targets.
        let mut scene_files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(self.project_dir.join("Scenes")) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("gxscene") {
                    scene_files.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }

        scene_files.sort();

        egui::SidePanel::right("gxui_inspector")
            .default_width(255.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Inspector");
                    ui.separator();

                    let selected = self.gui_editor.selected;

                    let canvas = &mut self.scene.canvas;

                    if let Some(widget) = canvas.widgets.iter_mut().find(|w| Some(w.id) == selected)
                    {
                        ui.label(format!("{:?}", widget.kind));

                        ui.text_edit_singleline(&mut widget.name);

                        ui.checkbox(&mut widget.visible, "Visible");

                        ui.separator();

                        ui.heading("Layout");

                        egui::ComboBox::from_id_salt("gxui_anchor")
                            .selected_text(ANCHORS[widget.anchor.min(8)])
                            .show_ui(ui, |ui| {
                                for (i, name) in ANCHORS.iter().enumerate() {
                                    ui.selectable_value(&mut widget.anchor, i, *name);
                                }
                            });

                        edit_vec2(ui, "Position", &mut widget.position);

                        edit_vec2(ui, "Size", &mut widget.size);

                        widget.size[0] = widget.size[0].max(10.0);

                        widget.size[1] = widget.size[1].max(10.0);

                        ui.separator();

                        ui.heading("Appearance");

                        ui.label("Background");

                        ui.color_edit_button_rgba_unmultiplied(&mut widget.color);

                        ui.label("Text Color");

                        ui.color_edit_button_rgba_unmultiplied(&mut widget.text_color);

                        ui.add(egui::Slider::new(&mut widget.radius, 0.0..=50.0).text("Radius"));

                        ui.label("Text");

                        ui.text_edit_multiline(&mut widget.text);

                        ui.add(
                            egui::Slider::new(&mut widget.font_size, 8.0..=150.0).text("Font Size"),
                        );

                        if widget.kind == WidgetKind::ProgressBar {
                            ui.add(
                                egui::Slider::new(&mut widget.value, 0.0..=1.0).text("Progress"),
                            );
                        }

                        if widget.kind == WidgetKind::Checkbox {
                            let mut checked = widget.value > 0.5;

                            ui.checkbox(&mut checked, "Checked");

                            widget.value = if checked { 1.0 } else { 0.0 };
                        }

                        if widget.kind == WidgetKind::Button {
                            ui.separator();
                            ui.heading("Events");

                            ui.label("On Clicked");

                            egui::ComboBox::from_id_salt("gxui_action")
                                .selected_text(format!("{:?}", widget.action))
                                .show_ui(ui, |ui| {
                                    for action in [
                                        ButtonAction::None,
                                        ButtonAction::LoadScene,
                                        ButtonAction::ToggleVisibility,
                                        ButtonAction::Quit,
                                    ] {
                                        ui.selectable_value(
                                            &mut widget.action,
                                            action,
                                            format!("{:?}", action),
                                        );
                                    }
                                });

                            if widget.action == ButtonAction::LoadScene {
                                egui::ComboBox::from_id_salt("gxui_scene_target")
                                    .selected_text(if widget.target.is_empty() {
                                        "Select Scene"
                                    } else {
                                        &widget.target
                                    })
                                    .show_ui(ui, |ui| {
                                        for name in &scene_files {
                                            ui.selectable_value(
                                                &mut widget.target,
                                                name.clone(),
                                                name,
                                            );
                                        }
                                    });
                            }

                            if widget.action == ButtonAction::ToggleVisibility {
                                ui.label("Target element name");

                                ui.text_edit_singleline(&mut widget.target);
                            }
                        }
                    } else {
                        ui.heading("Canvas");

                        ui.label(if canvas.ui_only {
                            "Scene Type: UI"
                        } else {
                            "Scene Type: 3D + UI"
                        });

                        ui.label(
                            "Disable the 3D viewport \
                                 for menus and UI scenes.",
                        );

                        edit_vec2(ui, "Reference Resolution", &mut canvas.resolution);

                        canvas.resolution[0] = canvas.resolution[0].clamp(320.0, 7680.0);

                        canvas.resolution[1] = canvas.resolution[1].clamp(240.0, 4320.0);

                        ui.label("Background");

                        ui.color_edit_button_rgba_unmultiplied(&mut canvas.background);
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("GXUI");

                ui.label(format!(
                    "{} × {}",
                    self.scene.canvas.resolution[0], self.scene.canvas.resolution[1],
                ));

                if ui.button("Main Menu Template").clicked() {
                    self.gui_menu_template();
                }
            });

            ui.separator();

            let (available, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());

            let available = available.shrink(18.0);

            let (canvas, scale) = canvas_rect(available, self.scene.canvas.resolution);

            let painter = ui.painter().with_clip_rect(canvas);

            painter.rect_filled(canvas, 0, color(self.scene.canvas.background));

            // Canvas grid.
            let spacing = 40.0 * scale;

            if spacing > 4.0 {
                let mut x = canvas.left();

                while x < canvas.right() {
                    painter.line_segment(
                        [egui::pos2(x, canvas.top()), egui::pos2(x, canvas.bottom())],
                        egui::Stroke::new(0.5_f32, egui::Color32::from_white_alpha(15)),
                    );

                    x += spacing;
                }

                let mut y = canvas.top();

                while y < canvas.bottom() {
                    painter.line_segment(
                        [egui::pos2(canvas.left(), y), egui::pos2(canvas.right(), y)],
                        egui::Stroke::new(0.5_f32, egui::Color32::from_white_alpha(15)),
                    );

                    y += spacing;
                }
            }

            for widget in &self.scene.canvas.widgets {
                if !widget.visible {
                    continue;
                }

                draw_widget(
                    &painter,
                    widget,
                    widget_rect(widget, canvas, scale),
                    scale,
                    false,
                );
            }

            // Select the topmost element under the mouse.
            let pointer = ctx.input(|i| i.pointer.interact_pos());

            if ctx.input(|i| i.pointer.primary_pressed()) {
                if let Some(pos) = pointer {
                    if canvas.contains(pos) {
                        let selected = self.gui_editor.selected;

                        let on_resize_handle = self
                            .scene
                            .canvas
                            .widgets
                            .iter()
                            .find(|w| Some(w.id) == selected)
                            .map(|w| {
                                let r = widget_rect(w, canvas, scale);

                                egui::Rect::from_center_size(
                                    r.right_bottom(),
                                    egui::vec2(16.0, 16.0),
                                )
                                .contains(pos)
                            })
                            .unwrap_or(false);

                        if !on_resize_handle {
                            self.gui_editor.selected = self
                                .scene
                                .canvas
                                .widgets
                                .iter()
                                .rev()
                                .find(|w| w.visible && widget_rect(w, canvas, scale).contains(pos))
                                .map(|w| w.id);
                        }
                    }
                }
            }

            if let Some(widget) = self
                .scene
                .canvas
                .widgets
                .iter_mut()
                .find(|w| Some(w.id) == self.gui_editor.selected)
            {
                let rect = widget_rect(widget, canvas, scale);

                let handle =
                    egui::Rect::from_center_size(rect.right_bottom(), egui::vec2(14.0, 14.0));

                let resize = ui.interact(
                    handle,
                    ui.id().with(("resize", widget.id)),
                    egui::Sense::drag(),
                );

                let movement =
                    ui.interact(rect, ui.id().with(("move", widget.id)), egui::Sense::drag());

                let delta = ctx.input(|i| i.pointer.delta()) / scale;

                if resize.dragged() {
                    widget.size[0] = (widget.size[0] + delta.x).max(10.0);

                    widget.size[1] = (widget.size[1] + delta.y).max(10.0);

                    let ax = (widget.anchor.min(8) % 3) as f32 * 0.5;

                    let ay = (widget.anchor.min(8) / 3) as f32 * 0.5;

                    widget.position[0] += delta.x * ax;
                    widget.position[1] += delta.y * ay;
                } else if movement.dragged() {
                    widget.position[0] += delta.x;
                    widget.position[1] += delta.y;
                }

                let rect = widget_rect(widget, canvas, scale);

                let selection_color = egui::Color32::from_rgb(100, 170, 255);

                for (a, b) in [
                    (rect.left_top(), rect.right_top()),
                    (rect.right_top(), rect.right_bottom()),
                    (rect.right_bottom(), rect.left_bottom()),
                    (rect.left_bottom(), rect.left_top()),
                ] {
                    painter.line_segment([a, b], egui::Stroke::new(1.5_f32, selection_color));
                }

                for point in [
                    rect.left_top(),
                    rect.right_top(),
                    rect.left_bottom(),
                    rect.right_bottom(),
                ] {
                    painter.rect_filled(
                        egui::Rect::from_center_size(point, egui::vec2(7.0, 7.0)),
                        1,
                        selection_color,
                    );
                }
            }
        });
    }
}

const ANCHORS: [&str; 9] = [
    "Top Left",
    "Top Center",
    "Top Right",
    "Middle Left",
    "Center",
    "Middle Right",
    "Bottom Left",
    "Bottom Center",
    "Bottom Right",
];

fn edit_vec2(ui: &mut egui::Ui, name: &str, values: &mut [f32; 2]) {
    ui.label(name);

    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut values[0]).prefix("X ").speed(1.0));

        ui.add(egui::DragValue::new(&mut values[1]).prefix("Y ").speed(1.0));
    });
}

// ==========================================
// MAIN MENU TEMPLATE
// ==========================================

impl GXEngine {
    fn gui_menu_template(&mut self) {
        let canvas = &mut self.scene.canvas;

        let id = canvas.add(WidgetKind::Label);

        if let Some(w) = canvas.widgets.iter_mut().find(|w| w.id == id) {
            w.name = "Game Title".into();
            w.text = "MY GAME".into();
            w.font_size = 84.0;
            w.position = [0.0, -240.0];
            w.size = [900.0, 140.0];
        }

        let id = canvas.add(WidgetKind::Button);

        if let Some(w) = canvas.widgets.iter_mut().find(|w| w.id == id) {
            w.name = "Play Button".into();
            w.text = "Play".into();
            w.position = [0.0, 20.0];
            w.size = [400.0, 85.0];
            w.action = ButtonAction::LoadScene;
        }

        let id = canvas.add(WidgetKind::Button);

        if let Some(w) = canvas.widgets.iter_mut().find(|w| w.id == id) {
            w.name = "Quit Button".into();
            w.text = "Quit".into();
            w.position = [0.0, 130.0];
            w.size = [400.0, 85.0];
            w.action = ButtonAction::Quit;
        }

        self.gui_editor.selected = None;
    }
}

// ==========================================
// RUNTIME UI
// ==========================================

impl GXEngine {
    pub(super) fn gui_hit(&self, rect: egui::Rect, pointer: egui::Pos2) -> bool {
        let (canvas, scale) = canvas_rect(rect, self.scene.canvas.resolution);

        self.scene.canvas.widgets.iter().rev().any(|w| {
            w.visible
                && matches!(w.kind, WidgetKind::Button | WidgetKind::Checkbox)
                && widget_rect(w, canvas, scale).contains(pointer)
        })
    }

    pub(super) fn gui_runtime(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let (canvas, scale) = canvas_rect(rect, self.scene.canvas.resolution);

        let painter = ui.painter().with_clip_rect(canvas);

        if self.scene.canvas.ui_only {
            ui.painter().rect_filled(rect, 0, egui::Color32::BLACK);

            painter.rect_filled(canvas, 0, color(self.scene.canvas.background));
        }

        let pointer = ui.input(|i| i.pointer.hover_pos());

        let interactive = self.playing && !self.mouse_locked;

        let hovered_id = if interactive {
            pointer.and_then(|pos| {
                self.scene
                    .canvas
                    .widgets
                    .iter()
                    .rev()
                    .find(|w| {
                        w.visible
                            && matches!(w.kind, WidgetKind::Button | WidgetKind::Checkbox)
                            && widget_rect(w, canvas, scale).contains(pos)
                    })
                    .map(|w| w.id)
            })
        } else {
            None
        };

        let clicked = interactive && ui.input(|i| i.pointer.primary_clicked());

        let mut action = None;

        for widget in &mut self.scene.canvas.widgets {
            if !widget.visible {
                continue;
            }

            let widget_rect = widget_rect(widget, canvas, scale);

            let hovered = Some(widget.id) == hovered_id;

            draw_widget(&painter, widget, widget_rect, scale, hovered);

            if hovered && clicked {
                if widget.kind == WidgetKind::Button {
                    action = Some((widget.action, widget.target.clone()));
                }

                if widget.kind == WidgetKind::Checkbox {
                    widget.value = if widget.value > 0.5 { 0.0 } else { 1.0 };
                }
            }
        }

        if let Some((action, target)) = action {
            match action {
                ButtonAction::None => {}

                ButtonAction::Quit => {
                    // Stop playtest, not the editor.
                    self.unlock_mouse(ui.ctx());
                    self.stop();
                }

                ButtonAction::ToggleVisibility => {
                    if let Some(widget) = self
                        .scene
                        .canvas
                        .widgets
                        .iter_mut()
                        .find(|w| w.name == target)
                    {
                        widget.visible = !widget.visible;
                    }
                }

                ButtonAction::LoadScene => {
                    self.gui_load_scene(ui.ctx(), &target);
                }
            }
        }
    }

    fn gui_load_scene(&mut self, ctx: &egui::Context, name: &str) {
        if !self.playing {
            return;
        }

        // Only allow project-local scene filenames.
        let path = std::path::Path::new(name);

        if path.file_name().and_then(|n| n.to_str()) != Some(name)
            || path.extension().and_then(|n| n.to_str()) != Some("gxscene")
        {
            self.status = "Invalid scene filename".into();

            return;
        }

        let path = self.project_dir.join("Scenes").join(name);

        let loaded = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|data| serde_json::from_str::<Scene>(&data).map_err(|e| e.to_string()));

        match loaded {
            Ok(mut scene) => {
                if let Err(error) = assets::hydrate_scene(&mut scene, &self.project_dir) {
                    self.status = error;
                    return;
                }
                scene.objects.truncate(MAX_OBJECTS);

                // Preserve the original editor backup.
                self.unlock_mouse(ctx);
                self.playing = false;

                // Clear previous scene script instances.
                self.scripting_update(ctx, 0.0);

                self.scene = scene;

                let mut world = physics::GXPhysics::new(&self.scene);

                if let Some(player) = self.scene.objects.iter().find(|o| o.player_mode != 0) {
                    world.spawn_player(player.position);

                    self.yaw = -std::f32::consts::FRAC_PI_2;

                    self.pitch = 0.0;
                }

                self.physics_world = Some(world);
                self.playing = true;

                self.status = format!("Scene loaded: {}", name);
            }

            Err(error) => {
                self.status = format!("Scene load failed: {}", error);
            }
        }
    }
}

use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Light {
    pub enabled: bool,
    pub kind: u8,
    pub color: [f32; 3],
    pub energy: f32,
    pub range: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            enabled: false,
            kind: 0,
            color: [1.0, 0.93, 0.82],
            energy: 30.0,
            range: 12.0,
        }
    }
}

impl GXEngine {
    pub(super) fn spawn_light(&mut self) {
        if self.playing || self.scene.objects.len() >= MAX_OBJECTS {
            return;
        }

        let mut object = Object::new(Shape::Cube);

        object.name = "Point Light".into();
        object.position = [0.0, 3.0, 0.0];
        object.scale = [0.2; 3];
        object.dynamic = false;

        object.light.enabled = true;
        object.light.kind = 0;

        object.emission = 0.0;
        object.light_only = true;

        object.light_only = true;
        object.emission = 0.0;

        self.scene.objects.push(object);

        self.selected = Some(self.scene.objects.len() - 1);

        self.environment_selected = false;
        self.status = "Point Light created".into();
    }
}

pub(super) fn light_inspector(ui: &mut egui::Ui, object: &mut Object) {
    ui.separator();
    ui.heading("Point Light");

    ui.checkbox(&mut object.light.enabled, "Enabled");

    if !object.light.enabled {
        return;
    }

    object.light.kind = 0;

    ui.label("Light Color");

    ui.color_edit_button_rgb(&mut object.light.color);

    ui.add(egui::Slider::new(&mut object.light.energy, 0.0..=200.0).text("Energy"));

    ui.add(egui::Slider::new(&mut object.light.range, 0.5..=50.0).text("Range"));
}

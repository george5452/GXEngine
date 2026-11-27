use super::*;

impl GXEngine {
    pub(super) fn graphics_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("GX Cinematic");
        ui.label("Project Rendering Settings");

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Save Graphics").clicked() {
                self.save_graphics_settings();
            }

            if ui.button("Load Graphics").clicked() {
                self.load_graphics_settings();
            }
        });

        let env = &mut self.scene.environment;

        ui.separator();

        ui.collapsing("Exposure & Lighting", |ui| {
            ui.add(egui::Slider::new(&mut env.exposure, 0.1..=3.0).text("Exposure"));

            ui.add(egui::Slider::new(&mut env.sun, 0.0..=8.0).text("Sun Energy"));
        });

        ui.collapsing("Global Illumination", |ui| {
            ui.add(egui::Slider::new(&mut env.gi, 0.0..=3.0).text("SDFGI Intensity"));

            ui.add(egui::Slider::new(&mut env.gi_distance, 1.0..=20.0).text("GI Distance"));
        });

        ui.collapsing("Shadows & Occlusion", |ui| {
            ui.add(egui::Slider::new(&mut env.ao, 0.0..=2.0).text("Ambient Occlusion"));

            ui.add(
                egui::Slider::new(&mut env.shadow_softness, 2.0..=64.0).text("Shadow Sharpness"),
            );
        });

        ui.collapsing("Reflections", |ui| {
            ui.add(egui::Slider::new(&mut env.reflections, 0.0..=1.0).text("Reflection Intensity"));
        });

        ui.collapsing("Atmosphere", |ui| {
            ui.add(egui::Slider::new(&mut env.fog, 0.0..=0.1).text("Fog Density"));

            ui.add(egui::Slider::new(&mut env.glow, 0.0..=2.0).text("Emissive Intensity"));
        });

        ui.collapsing("World Physics", |ui| {
            ui.add(egui::Slider::new(&mut env.gravity, 0.0..=30.0).text("Gravity"));
        });

        ui.separator();

        if ui.button("Natural Daylight").clicked() {
            env.exposure = 1.0;
            env.sun = 3.5;
            env.gi = 0.65;
            env.gi_distance = 6.0;
            env.ao = 0.7;
            env.shadow_softness = 18.0;
            env.reflections = 0.8;
            env.fog = 0.002;
            env.glow = 0.15;
        }

        if ui.button("Reset Graphics").clicked() {
            let gravity = env.gravity;
            *env = Environment::default();
            env.gravity = gravity;
        }
    }

    pub(super) fn save_graphics_settings(&mut self) {
        let path = self.project_dir.join("graphics.json");

        let result = (|| -> Result<(), String> {
            std::fs::create_dir_all(&self.project_dir).map_err(|e| e.to_string())?;

            let json =
                serde_json::to_string_pretty(&self.scene.environment).map_err(|e| e.to_string())?;

            std::fs::write(&path, json).map_err(|e| e.to_string())?;

            Ok(())
        })();

        self.status = match result {
            Ok(_) => "Project graphics saved".into(),
            Err(e) => format!("Graphics save failed: {}", e),
        };
    }

    pub(super) fn load_graphics_settings(&mut self) {
        let path = self.project_dir.join("graphics.json");

        let result = (|| -> Result<Environment, String> {
            let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;

            serde_json::from_str::<Environment>(&json).map_err(|e| e.to_string())
        })();

        match result {
            Ok(environment) => {
                self.scene.environment = environment;
                self.status = "Project graphics loaded".into();
            }

            Err(e) => {
                self.status = format!("Graphics load failed: {}", e);
            }
        }
    }
}

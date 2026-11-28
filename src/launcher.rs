use super::*;

impl GXEngine {
    fn create_project(&mut self) -> Result<(), String> {
        let name = self.project_name.trim();

        if name.is_empty()
            || name == "."
            || name == ".."
            || name
                .chars()
                .any(|c| !c.is_alphanumeric() && c != ' ' && c != '_' && c != '-')
        {
            return Err("Invalid project name".into());
        }

        let root = PathBuf::from("projects").join(name);

        if root.exists() {
            return Err("Project already exists".into());
        }

        for folder in [
            "Assets",
            "Assets/Models",
            "Assets/Materials",
            "Assets/Textures",
            "Assets/Audio",
            "Assets/Scripts",
            "Scenes",
        ] {
            std::fs::create_dir_all(root.join(folder)).map_err(|e| e.to_string())?;
        }

        self.scene = Scene::default();

        match self.template {
            0 => {
                self.scene.objects.clear();
            }

            1 => {}

            2 => {
                self.scene.objects.clear();

                for y in 0..5 {
                    for x in 0..4 {
                        let mut object = Object::new(Shape::Cube);

                        object.name = format!("Physics Cube {} {}", x, y);

                        object.position = [x as f32 * 1.2 - 2.0, y as f32 * 1.2 + 0.5, 0.0];

                        self.scene.objects.push(object);
                    }
                }
            }

            _ => {}
        }

        let path = root.join("Scenes/Main.gxscene");

        let data = serde_json::to_string_pretty(&self.scene).map_err(|e| e.to_string())?;

        std::fs::write(&path, data).map_err(|e| e.to_string())?;

        self.path = path.to_string_lossy().into_owned();
        self.project_dir = root.clone();
        self.browser_dir = root;

        self.selected = None;
        self.environment_selected = false;
        self.playing = false;
        self.launcher_open = false;

        self.save_graphics_settings();

        Ok(())
    }

    fn open_project(&mut self, path: PathBuf) {
        let scene_path = path.join("Scenes/Main.gxscene");

        let result = std::fs::read_to_string(&scene_path)
            .map_err(|e| e.to_string())
            .and_then(|data| serde_json::from_str::<Scene>(&data).map_err(|e| e.to_string()));

        match result {
            Ok(mut scene) => {
                scene.objects.truncate(MAX_OBJECTS);

                self.scene = scene;
                self.path = scene_path.to_string_lossy().into_owned();
                self.scripter = scripting::GXScripter::default();

                self.project_dir = path.clone();
                self.browser_dir = path;

                self.selected = None;
                self.environment_selected = false;
                self.playing = false;
                self.launcher_open = false;

                if self.project_dir.join("graphics.json").exists() {
                    self.load_graphics_settings();
                } else {
                    self.status = "Project opened".into();
                }
            }

            Err(error) => {
                self.status = error;
            }
        }
    }

    pub(super) fn show_launcher(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(30.0);

            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("GXEngine")
                        .size(48.0)
                        .strong()
                        .color(egui::Color32::from_rgb(190, 155, 255)),
                );

                ui.label("Create something extraordinary.");
                ui.label("Version 0.3");
            });

            ui.add_space(30.0);
            ui.separator();

            ui.columns(2, |columns| {
                let ui = &mut columns[0];

                ui.heading("New Project");
                ui.add_space(12.0);

                ui.label("Project name");
                ui.text_edit_singleline(&mut self.project_name);

                ui.add_space(15.0);
                ui.heading("Starter Templates");

                for (index, title, desc) in [
                    (0, "Empty", "Start completely from scratch"),
                    (1, "3D Starter", "Cinematic environment and primitives"),
                    (2, "Physics Sandbox", "Spawn a stack of physics objects"),
                ] {
                    ui.group(|ui| {
                        ui.selectable_value(&mut self.template, index, title);

                        ui.label(desc);
                    });

                    ui.add_space(8.0);
                }

                if ui
                    .add_sized([200.0, 40.0], egui::Button::new("Create Project"))
                    .clicked()
                {
                    if let Err(error) = self.create_project() {
                        self.status = error;
                    }
                }

                let ui = &mut columns[1];

                ui.heading("Existing Projects");
                ui.add_space(12.0);

                if let Ok(entries) = std::fs::read_dir("projects") {
                    for entry in entries.flatten() {
                        let path = entry.path();

                        if path.is_dir() && path.join("Scenes/Main.gxscene").exists() {
                            let name = entry.file_name().to_string_lossy().into_owned();

                            if ui
                                .add_sized([240.0, 44.0], egui::Button::new(name))
                                .clicked()
                            {
                                self.open_project(path);
                            }
                        }
                    }
                }
            });

            ui.separator();
            ui.label(&self.status);
        });
    }
}

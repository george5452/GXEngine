use super::*;

#[derive(Default)]
pub(super) struct SceneManager {
    new_name: String,
    scene_type: usize,
}

impl GXEngine {
    fn save_current_scene(&mut self) -> Result<(), String> {
        if self.playing {
            return Err("Stop the game before saving".into());
        }

        self.scripter.save_pending(&self.project_dir)?;

        assets::save_scene(
            &self.scene,
            std::path::Path::new(&self.path),
            &self.project_dir,
        )
    }

    fn create_scene(&mut self, name: &str, ui_only: bool) -> Result<(), String> {
        let name = name.trim();

        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ')
        {
            return Err("Invalid scene name".into());
        }

        let directory = self.project_dir.join("Scenes");

        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;

        let path = directory.join(format!("{}.gxscene", name));

        if path.exists() {
            return Err("Scene already exists".into());
        }

        // Save the current scene before switching.
        self.save_current_scene()?;

        let mut scene = Scene::default();

        scene.objects.clear();
        scene.environment = self.scene.environment.clone();
        scene.canvas.ui_only = ui_only;

        let data = serde_json::to_string_pretty(&scene).map_err(|e| e.to_string())?;

        std::fs::write(&path, data).map_err(|e| e.to_string())?;

        self.scene = scene;

        self.path = path.to_string_lossy().into_owned();

        self.selected = None;
        self.environment_selected = false;

        self.gui_editor = gui::GUIEditor::default();
        self.gui_editor.open = ui_only;

        self.status = format!("Created scene: {}", name);

        Ok(())
    }

    pub(super) fn open_scene_file(&mut self, path: PathBuf) -> Result<(), String> {
        if self.playing {
            return Err("Stop the game first".into());
        }

        if path == PathBuf::from(&self.path) {
            return Ok(());
        }

        // Validate the destination before changing anything.
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;

        let mut scene: Scene = serde_json::from_str(&data).map_err(|e| e.to_string())?;

        assets::hydrate_scene(&mut scene, &self.project_dir)?;
        scene.objects.truncate(MAX_OBJECTS);

        self.save_current_scene()?;

        self.scene = scene;
        self.path = path.to_string_lossy().into_owned();

        self.selected = None;
        self.environment_selected = false;

        self.gui_editor = gui::GUIEditor::default();

        self.gui_editor.open = self.scene.canvas.ui_only;

        self.status = format!(
            "Scene opened: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );

        Ok(())
    }

    pub(super) fn scene_toolbar(&mut self, ui: &mut egui::Ui) {
        let current = PathBuf::from(&self.path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        ui.strong(current);

        ui.separator();

        ui.add_enabled_ui(!self.playing, |ui| {
            ui.menu_button("Scenes", |ui| {
                ui.heading("Project Scenes");

                let directory = self.project_dir.join("Scenes");

                let mut paths = Vec::new();

                if let Ok(entries) = std::fs::read_dir(directory) {
                    for entry in entries.flatten() {
                        let path = entry.path();

                        if path.extension().and_then(|e| e.to_str()) == Some("gxscene") {
                            paths.push(path);
                        }
                    }
                }

                paths.sort();

                for path in paths {
                    let name = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();

                    let active = path == PathBuf::from(&self.path);

                    if ui.selectable_label(active, name).clicked() {
                        if let Err(error) = self.open_scene_file(path) {
                            self.status = error;
                        }

                        ui.close_menu();
                    }
                }

                ui.separator();
                ui.heading("New Scene");

                ui.label("Scene Name");

                ui.text_edit_singleline(&mut self.scene_manager.new_name);

                ui.radio_value(&mut self.scene_manager.scene_type, 0, "3D Scene");

                ui.radio_value(&mut self.scene_manager.scene_type, 1, "UI Scene");

                if ui.button("Create Scene").clicked() {
                    let name = self.scene_manager.new_name.clone();

                    let ui_only = self.scene_manager.scene_type == 1;

                    match self.create_scene(&name, ui_only) {
                        Ok(()) => {
                            self.scene_manager.new_name.clear();

                            ui.close_menu();
                        }

                        Err(error) => {
                            self.status = error;
                        }
                    }
                }
            });
        });
    }
}

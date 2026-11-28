use super::*;

impl GXEngine {
    pub(super) fn show_browser(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("content_browser")
            .resizable(true)
            .default_height(210.0)
            .min_height(100.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.browser_tab, 0, "Content Browser");

                    ui.selectable_value(&mut self.browser_tab, 1, "GXStarter");

                    ui.selectable_value(&mut self.browser_tab, 2, "Console");
                });

                ui.separator();

                match self.browser_tab {
                    0 => self.show_project_files(ui),
                    1 => self.show_starter(ui),
                    2 => {
                        ui.monospace(&self.status);
                        ui.label("GXEngine Native Runtime");
                        ui.label("Renderer: Cinematic");
                        ui.label(format!("FPS: {:.0}", self.fps));
                    }
                    _ => {}
                }
            });
    }

    fn show_project_files(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("+ Import Model").clicked() && !self.playing {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("3D Models", &["glb", "gltf"])
                    .pick_file()
                {
                    if path.extension().and_then(|s| s.to_str()) == Some("glb") {
                        let directory = self.project_dir.join("Assets").join("Models");

                        let _ = std::fs::create_dir_all(&directory);

                        if let Some(name) = path.file_name() {
                            let destination = directory.join(name);

                            if !destination.exists() {
                                let _ = std::fs::copy(&path, &destination);
                            }

                            self.browser_dir = directory;
                        }
                    }

                    self.import_model(&path);
                }
            }
            if ui.button("Home").clicked() {
                self.browser_dir = self.project_dir.clone();
            }

            if ui.button("Up").clicked() {
                if self.browser_dir != self.project_dir {
                    if let Some(parent) = self.browser_dir.parent() {
                        if parent.starts_with(&self.project_dir) {
                            self.browser_dir = parent.to_path_buf();
                        }
                    }
                }
            }

            if ui.button("+ Folder").clicked() {
                for i in 1..1000 {
                    let path = self.browser_dir.join(format!("Folder {}", i));

                    if !path.exists() {
                        match std::fs::create_dir(path) {
                            Ok(_) => {
                                self.status = "Folder created".into();
                            }

                            Err(e) => {
                                self.status = e.to_string();
                            }
                        }

                        break;
                    }
                }
            }

            if ui.button("Save Scene").clicked() && !self.playing {
                self.save();
            }

            ui.label(self.browser_dir.to_string_lossy());
        });

        ui.separator();

        let mut files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.browser_dir) {
            for entry in entries.flatten() {
                files.push(entry.path());
            }
        }

        files.sort_by_key(|p| (!p.is_dir(), p.clone()));

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for path in files {
                    let folder = path.is_dir();

                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();

                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

                    let category = if folder {
                        "FOLDER"
                    } else {
                        match ext {
                            "gxscene" => "SCENE",
                            "glb" | "gltf" => "MODEL",
                            "png" | "jpg" => "TEXTURE",
                            "wav" | "mp3" => "AUDIO",
                            "rs" | "rhai" => "SCRIPT",
                            _ => "FILE",
                        }
                    };

                    let response = ui.add_sized(
                        [115.0, 70.0],
                        egui::Button::new(format!("{}\n{}", category, name)),
                    );

                    if response.double_clicked() {
                        if folder {
                            self.browser_dir = path.clone();
                        } else if matches!(ext, "glb" | "gltf") && !self.playing {
                            self.import_model(&path);
                        } else if ext == "gxscene" && !self.playing {
                            if let Err(error) = self.open_scene_file(path.clone()) {
                                self.status = error;
                            }
                        } else {
                            self.status = format!("Selected {}", name);
                        }
                    }
                }
            });
        });
    }
}

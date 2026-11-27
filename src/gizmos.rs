use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum GizmoMode {
    Select,
    Move,
    Rotate,
    Scale,
}

pub(super) struct GizmoDrag {
    axis: usize,
    direction: egui::Vec2,
    pixels_per_unit: f32,
}

impl GXEngine {
    fn project_point(&self, point: Vec3, rect: egui::Rect) -> Option<egui::Pos2> {
        let forward = self.forward();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();

        let relative = point - self.camera;

        let depth = relative.dot(forward);

        if depth <= 0.05 {
            return None;
        }

        let fov = 0.46630766;

        let factor = rect.height() / (2.0 * fov);

        let x = relative.dot(right) / depth * factor;
        let y = relative.dot(up) / depth * factor;

        Some(rect.center() + egui::vec2(x, -y))
    }

    pub(super) fn gizmo_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Select, "Select");

        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Move, "Move");

        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Rotate, "Rotate");

        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Scale, "Scale");
    }

    pub(super) fn draw_gizmos(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        if self.playing {
            self.gizmo_drag = None;
            return;
        }

        let ctx = ui.ctx();

        let pointer = ctx.input(|i| i.pointer.interact_pos());

        let pressed = ctx.input(|i| i.pointer.primary_pressed());

        let down = ctx.input(|i| i.pointer.primary_down());

        let delta = ctx.input(|i| i.pointer.delta());

        if !down {
            self.gizmo_drag = None;
        }

        let mut handled = self.gizmo_drag.is_some();

        if let Some(index) = self.selected {
            if index < self.scene.objects.len() {
                let position = Vec3::from_array(self.scene.objects[index].position);

                if let Some(center) = self.project_point(position, rect) {
                    if rect.contains(center) && self.gizmo_mode != GizmoMode::Select {
                        let axes = [Vec3::X, Vec3::Y, Vec3::Z];

                        let colors = [
                            egui::Color32::from_rgb(255, 75, 75),
                            egui::Color32::from_rgb(75, 240, 100),
                            egui::Color32::from_rgb(75, 140, 255),
                        ];

                        for axis in 0..3 {
                            let Some(world_end) = self.project_point(position + axes[axis], rect)
                            else {
                                continue;
                            };

                            let projected = world_end - center;

                            let length = projected.length();

                            if length < 2.0 {
                                continue;
                            }

                            let direction = projected / length;

                            let end = center + direction * 75.0;

                            let hovering = pointer
                                .map(|p| {
                                    let offset = p - center;

                                    let along = offset.dot(direction).clamp(0.0, 75.0);

                                    let closest = center + direction * along;

                                    p.distance(closest) < 9.0 && along > 12.0 && rect.contains(p)
                                })
                                .unwrap_or(false);

                            let color = if hovering {
                                egui::Color32::YELLOW
                            } else {
                                colors[axis]
                            };

                            ui.painter()
                                .line_segment([center, end], egui::Stroke::new(3.0_f32, color));

                            match self.gizmo_mode {
                                GizmoMode::Move => {
                                    ui.painter().arrow(
                                        center,
                                        end - center,
                                        egui::Stroke::new(3.0_f32, color),
                                    );
                                }

                                GizmoMode::Scale => {
                                    ui.painter().rect_filled(
                                        egui::Rect::from_center_size(end, egui::vec2(11.0, 11.0)),
                                        1.0,
                                        color,
                                    );
                                }

                                GizmoMode::Rotate => {
                                    ui.painter().circle_stroke(
                                        end,
                                        7.0,
                                        egui::Stroke::new(3.0_f32, color),
                                    );
                                }

                                _ => {}
                            }

                            ui.painter().text(
                                end + direction * 12.0,
                                egui::Align2::CENTER_CENTER,
                                ["X", "Y", "Z"][axis],
                                egui::FontId::proportional(12.0),
                                color,
                            );

                            if hovering && pressed && !handled {
                                self.gizmo_drag = Some(GizmoDrag {
                                    axis,
                                    direction,
                                    pixels_per_unit: length.max(5.0),
                                });

                                handled = true;
                            }
                        }
                    }
                }

                if down {
                    if let Some(drag) = &self.gizmo_drag {
                        let movement = delta.dot(drag.direction);

                        let amount = movement / drag.pixels_per_unit;

                        let object = &mut self.scene.objects[index];

                        match self.gizmo_mode {
                            GizmoMode::Move => {
                                object.position[drag.axis] += amount;
                            }

                            GizmoMode::Rotate => {
                                object.rotation[drag.axis] += movement * 0.7;
                            }

                            GizmoMode::Scale => {
                                object.scale[drag.axis] =
                                    (object.scale[drag.axis] + amount).max(0.05);
                            }

                            _ => {}
                        }
                    }
                }
            }
        }

        // Basic viewport object selection.
        if pressed && !handled {
            if let Some(pointer) = pointer {
                if rect.contains(pointer) {
                    let mut nearest = None;
                    let mut distance = 65.0;

                    for (i, object) in self.scene.objects.iter().enumerate() {
                        let position = Vec3::from_array(object.position);

                        if let Some(point) = self.project_point(position, rect) {
                            let d = point.distance(pointer);

                            if d < distance {
                                distance = d;
                                nearest = Some(i);
                            }
                        }
                    }

                    self.selected = nearest;
                    self.environment_selected = false;
                }
            }
        }
    }
}

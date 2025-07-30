use eframe::egui;
use crate::winget_manager::open_url; // Importez open_url
use super::app::PackageApp; // Importez PackageApp depuis le module parent

impl eframe::App for PackageApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Appliquer les styles
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.visuals.resize_corner_size = 10.0.into();
        style.visuals.code_bg_color = egui::Color32::from_rgb(45, 45, 45);
        style.visuals.window_fill = egui::Color32::from_rgb(32, 32, 32);
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Titre et Refresh
                self.draw_header_and_refresh_button(ui);
                ui.add_space(10.0);

                // Gestion des promesses et affichage des erreurs
                self.handle_fetch_promise(); // Appelle la méthode déplacée dans app.rs
                self.handle_update_promise(); // Appelle la méthode déplacée dans app.rs

                self.draw_error_message(ui);

                // Liste des paquets
                self.draw_package_list(ui);

                // Logique de traitement de la queue (déplacée dans PackageApp)
                // self.process_update_queue(); // Ceci est appelé après chaque mise à jour de promesse
            });
        });

        // Demande de rafraîchissement si des promesses sont en cours
        if self.fetch_promise.is_some() || self.update_promise.is_some() {
            ctx.request_repaint();
        }
    }
}

// Méthodes d'aide pour le dessin de l'UI (peuvent être privées ou publiques selon besoin)
impl PackageApp {
    fn draw_header_and_refresh_button(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("Gestionnaire de Mises à Jour (winget)")
                .size(24.0)
                .color(egui::Color32::from_rgb(200, 200, 200)));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.update_queue.is_empty() {
                    ui.horizontal(|ui| {
                        if ui.add_sized(
                            [150.0, 30.0],
                            egui::Button::new(
                                egui::RichText::new(
                                    if self.is_batch_updating {
                                        format!("En cours ({} restants)", self.update_queue.len() + 1)
                                    } else {
                                        format!("Installer {} paquets", self.update_queue.len())
                                    }
                                ).size(16.0)
                            ).fill(egui::Color32::from_rgb(34, 197, 94))
                        ).clicked() && !self.is_batch_updating {
                            self.start_batch_update();
                        }

                        if ui.add_sized(
                            [100.0, 30.0],
                            egui::Button::new(
                                egui::RichText::new("Annuler")
                                    .size(16.0)
                            ).fill(egui::Color32::from_rgb(239, 68, 68))
                        ).clicked() {
                            self.cancel_batch_update();
                        }
                    });
                }
                let refresh_button = ui.add_sized(
                    [120.0, 30.0],
                    egui::Button::new(
                        egui::RichText::new(
                            if self.is_loading {
                                "⟳ Chargement..."
                            } else {
                                "⟳ Rafraîchir"
                            }
                        ).size(16.0)
                    ).fill(if self.is_loading {
                        egui::Color32::from_rgb(70, 70, 70)
                    } else {
                        egui::Color32::from_rgb(59, 130, 246)
                    })
                );

                if refresh_button.clicked() && !self.is_loading {
                    self.fetch_updates_async();
                }

                if self.is_loading {
                    ui.spinner();
                }
            });
        });
    }

    fn draw_error_message(&self, ui: &mut egui::Ui) {
        if !self.error_message.is_empty() {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(153, 27, 27))
                .corner_radius(8.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.colored_label(
                        egui::Color32::WHITE,
                        format!("⚠ {}", self.error_message)
                    );
                });
            ui.add_space(10.0);
        }
    }

    fn draw_package_list(&mut self, ui: &mut egui::Ui) {
        let mut queue_updates = Vec::new();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for package in &self.packages {
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(45, 45, 45))
                        .corner_radius(8.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let is_in_queue = self.update_queue.contains(&package.id);
                                let mut checked = is_in_queue;

                                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                    if ui.checkbox(&mut checked, "").changed() {
                                        if checked {
                                            queue_updates.push((package.id.clone(), true));
                                        } else {
                                            queue_updates.push((package.id.clone(), false));
                                        }
                                    }
                                });

                                ui.vertical(|ui| {
                                    ui.heading(
                                        egui::RichText::new(&package.name)
                                            .size(18.0)
                                            .color(egui::Color32::from_rgb(200, 200, 200))
                                    );
                                    ui.label(
                                        egui::RichText::new(&package.id)
                                            .color(egui::Color32::from_rgb(150, 150, 150))
                                    );
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add_sized(
                                        [100.0, 30.0],
                                        egui::Button::new(
                                            egui::RichText::new("Rechercher")
                                                .color(egui::Color32::WHITE)
                                        ).fill(egui::Color32::from_rgb(59, 130, 246))
                                    ).clicked() {
                                        let search_url = format!(
                                            "https://www.google.com/search?q={}+software+download",
                                            package.name.replace(" ", "+")
                                        );
                                        open_url(&search_url);
                                    }

                                    let is_updating_this_package = self.updating_package_id.as_ref() == Some(&package.id);
                                    let in_queue = self.update_queue.contains(&package.id);

                                    let update_button = ui.add_sized(
                                        [120.0, 30.0],
                                        egui::Button::new(
                                            egui::RichText::new(
                                                if is_updating_this_package {
                                                    "Mise à jour..."
                                                } else if in_queue {
                                                    "En attente"
                                                } else {
                                                    "Installer"
                                                }
                                            ).color(egui::Color32::BLACK)
                                        ).fill(if is_updating_this_package {
                                            egui::Color32::from_rgb(70, 70, 70)
                                        } else if in_queue {
                                            egui::Color32::from_rgb(167, 139, 250)
                                        } else {
                                            egui::Color32::from_rgb(234, 179, 8)
                                        })
                                    );

                                    if update_button.clicked() && !is_updating_this_package && !in_queue {
                                        queue_updates.push((package.id.clone(), true));
                                        queue_updates.push((String::new(), false)); // Signal pour start_batch_update
                                    }

                                    if is_updating_this_package {
                                        ui.spinner();
                                    }

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&package.available_version)
                                                .color(if package.version != package.available_version {
                                                    egui::Color32::from_rgb(234, 179, 8)
                                                } else {
                                                    egui::Color32::from_rgb(34, 197, 94)
                                                })
                                                .size(16.0)
                                        );

                                        ui.label(
                                            egui::RichText::new(" to ")
                                                .color(egui::Color32::from_rgb(100, 100, 100))
                                                .size(16.0)
                                        );

                                        ui.label(
                                            egui::RichText::new(&package.version)
                                                .color(egui::Color32::from_rgb(150, 150, 150))
                                                .size(16.0)
                                        );
                                    });
                                });
                            });
                        });
                }
            });

        for (id, add) in queue_updates {
            if id.is_empty() {
                self.start_batch_update();
            } else if add {
                self.add_to_update_queue(id);
            } else {
                self.update_queue.retain(|queue_id| queue_id != &id);
            }
        }
    }
}
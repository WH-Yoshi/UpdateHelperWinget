use crate::winget_manager::{Package, WingetManager, open_url};
use eframe::egui;
use poll_promise::Promise;

pub struct PackageApp {
    packages: Vec<Package>,
    error_message: String,
    promise: Option<Promise<Result<Vec<Package>, String>>>,
    is_loading: bool,
}

impl Default for PackageApp {
    fn default() -> Self {
        Self {
            packages: Vec::new(),
            error_message: String::new(),
            promise: None,
            is_loading: false,
        }
    }
}

impl PackageApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn fetch_updates_async(&mut self) {
        if self.promise.is_some() || self.is_loading {
            return;
        }

        self.is_loading = true;

        let promise = Promise::spawn_thread("winget_fetch", || {
            let rx = WingetManager::fetch_updates_async();
            rx.recv().unwrap_or(Err("Erreur de communication avec le thread".to_string()))
        });

        self.promise = Some(promise);
    }
}

impl eframe::App for PackageApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.visuals.resize_corner_size = 10.0.into();
        style.visuals.code_bg_color = egui::Color32::from_rgb(45, 45, 45);
        style.visuals.window_fill = egui::Color32::from_rgb(32, 32, 32);
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // En-tête avec titre et bouton
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Gestionnaire de Mises à Jour (winget)")
                        .size(24.0)
                        .color(egui::Color32::from_rgb(200, 200, 200)));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let refresh_button = ui.add_sized(
                            [120.0, 30.0],
                            egui::Button::new(
                                egui::RichText::new(
                                    if self.is_loading {
                                        "⟳ Chargement..."
                                    } else {
                                        "⟳ Rafraîchir"
                                    }
                                )
                                    .size(16.0)
                            )
                                .fill(if self.is_loading {
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

                ui.add_space(10.0);

                if let Some(promise) = &self.promise {
                    if let Some(result) = promise.ready() {
                        match result {
                            Ok(packages) => {
                                self.packages = packages.clone();
                                self.error_message.clear();
                            }
                            Err(error) => {
                                self.error_message = error.clone();
                                self.packages.clear();
                            }
                        }
                        self.promise = None;
                        self.is_loading = false;
                    } else {
                        ctx.request_repaint();
                    }
                }

                // Message d'erreur
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

                // Liste des packages
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
                                                egui::Button::new("Rechercher")
                                                    .fill(egui::Color32::from_rgb(59, 130, 246))
                                            ).clicked() {
                                                let search_url = format!(
                                                    "https://www.google.com/search?q={}+software+download",
                                                    package.name.replace(" ", "+")
                                                );
                                                open_url(&search_url);
                                            }

                                            ui.label(
                                                egui::RichText::new(
                                                    format!("{} → {}", package.version, package.available_version)
                                                )
                                                    .color(if package.version != package.available_version {
                                                        egui::Color32::from_rgb(234, 179, 8)
                                                    } else {
                                                        egui::Color32::from_rgb(34, 197, 94)
                                                    })
                                                    .size(16.0)
                                            );
                                        });
                                    });
                                });
                        }
                    });
            });
        });
    }
}
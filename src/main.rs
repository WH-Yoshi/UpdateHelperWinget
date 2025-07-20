use std::process::Command;
use std::sync::mpsc;
use std::thread;
use egui::ViewportBuilder;
use poll_promise::Promise;

#[derive(Debug, Clone)]
struct Package {
    name: String,
    id: String,
    version: String,
    available_version: String,
}

struct PackageApp {
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
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn fetch_updates_async(&mut self) {
        if self.promise.is_some() || self.is_loading {
            return; // Évite les multiples requêtes
        }

        self.is_loading = true;

        let promise = Promise::spawn_thread("winget_fetch", move || {
            let (tx, rx) = mpsc::channel();

            thread::spawn(move || {
                let output = Command::new("winget")
                    .args(["upgrade", "--include-unknown"])
                    .output();

                match output {
                    Ok(output) => {
                        if output.status.success() {
                            if let Ok(text) = String::from_utf8(output.stdout) {
                                let mut packages = Vec::new();
                                let lines: Vec<&str> = text.lines().collect();

                                if lines.len() > 2 {
                                    for line in lines[2..].iter() {
                                        if line.trim().is_empty() || line.starts_with('-') {
                                            continue;
                                        } else if line.ends_with("disponibles.") {
                                            break;
                                        }

                                        let columns: Vec<&str> = line.split_whitespace().collect();

                                        if columns.len() >= 4 {
                                            let len = columns.len();
                                            let available_version = columns[len - 2];
                                            let version = columns[len - 3];
                                            let id = columns[len - 4];
                                            let name = columns[..(len - 4)].join(" ");

                                            packages.push(Package {
                                                name,
                                                id: id.to_string(),
                                                version: version.to_string(),
                                                available_version: available_version.to_string(),
                                            });
                                        }
                                    }
                                    tx.send(Ok(packages)).ok();
                                }
                            }
                        } else {
                            let error = String::from_utf8_lossy(&output.stderr).to_string();
                            tx.send(Err(error)).ok();
                        }
                    }
                    Err(e) => {
                        tx.send(Err(format!("Erreur lors de l'exécution de winget: {}", e))).ok();
                    }
                }
            });

            rx.recv().unwrap_or(Err("Erreur de communication avec le thread".to_string()))
        });

        self.promise = Some(promise);
    }
}

impl eframe::App for PackageApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Style personnalisé
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
                    ui.heading(egui::RichText::new("Gestionnaire de Mises à Jour")
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

                // Message d'erreur
                if !self.error_message.is_empty() {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(153, 27, 27))
                        .rounding(8.0)
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
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(45, 45, 45))
                                .rounding(8.0)
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

        if self.is_loading {
            ctx.request_repaint();
        }
    }
}


fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .ok();
    }
}


fn main() -> Result<(), eframe::Error> {
    let viewport = ViewportBuilder::default()
        .with_inner_size([800.0, 600.0])
        .with_min_inner_size([400.0, 300.0])
        .with_maximize_button(true)
        .with_minimize_button(true)
        .with_close_button(true)
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_title("Gestionnaire de Mises à Jour");

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Gestionnaire de mises à jour",
        native_options,
        Box::new(|cc| Ok(Box::new(PackageApp::new(cc))))
    )
}

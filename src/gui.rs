use crate::winget_manager::{open_url, CancellationToken, InstallResult, Package, WingetError, WingetManager};
use eframe::egui::{vec2, Align, Button, CentralPanel, Color32, Context, Frame, Layout, RichText, ScrollArea};
use poll_promise::Promise;

pub struct PackageApp {
    packages: Vec<Package>,
    error: Option<WingetError>,
    install_result: Option<InstallResult>,
    fetch_promise: Option<Promise<Result<Vec<Package>, WingetError>>>,
    install_promise: Option<Promise<Result<InstallResult, WingetError>>>,
    is_loading: bool,
    updating_package_id: Option<String>,
    cancel_token: Option<CancellationToken>,
    cancel_clicked: bool,
}

impl Default for PackageApp {
    fn default() -> Self {
        Self {
            packages: Vec::new(),
            error: None,
            install_result: None,
            fetch_promise: None,
            install_promise: None,
            is_loading: false,
            updating_package_id: None,
            cancel_token: None,
            cancel_clicked: false,
        }
    }
}

impl PackageApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.fetch_updates_async();
        app
    }

    fn fetch_updates_async(&mut self) {
        if self.fetch_promise.is_some() || self.is_loading {
            return;
        }

        self.is_loading = true;

        let promise = Promise::spawn_thread("winget_fetch", || {
            let rx = WingetManager::fetch_updates_async();
            rx.recv().unwrap_or(Err(WingetError::CommandFailed { 
                error: "Erreur de communication avec le thread".to_string() 
            }))
        });

        self.fetch_promise = Some(promise);
    }

    fn update_single_async(&mut self, package_id: &str) {
        if self.install_promise.is_some() || self.updating_package_id.is_some() {
            return;
        }

        self.updating_package_id = Some(package_id.to_string());
        let cancel_token = CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());
        self.cancel_clicked = false;

        let package_id = package_id.to_string();
        let promise = Promise::spawn_thread("winget_install_single", move || {
            let rx = WingetManager::install_single(&package_id, cancel_token);
            rx.recv().unwrap_or(Err(WingetError::CommandFailed { 
                error: "Erreur de communication avec le thread".to_string() 
            }))
        });

        self.install_promise = Some(promise);
    }
}

impl eframe::App for PackageApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = vec2(10.0, 10.0);
        style.visuals.resize_corner_size = 10.0.into();
        style.visuals.code_bg_color = Color32::from_rgb(45, 45, 45);
        style.visuals.window_fill = Color32::from_rgb(32, 32, 32);
        ctx.set_style(style);

        CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Gestionnaire de Mises à Jour (winget)")
                        .size(24.0)
                        .color(Color32::from_rgb(200, 200, 200)));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let refresh_button = ui.add_sized(
                            [120.0, 30.0],
                            Button::new(
                                RichText::new(
                                    if self.is_loading {
                                        "⟳ Chargement..."
                                    } else {
                                        "⟳ Rafraîchir"
                                    }
                                )
                                    .size(16.0)
                                    .color(Color32::WHITE)
                            )
                                .fill(if self.is_loading {
                                    Color32::from_rgb(70, 70, 70)
                                } else {
                                    Color32::from_rgb(59, 130, 246)
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

                // Handle fetch promise
                if let Some(promise) = &self.fetch_promise {
                    if let Some(result) = promise.ready() {
                        self.error = None;
                        match result {
                            Ok(packages) => {
                                self.packages = packages.clone();
                            }
                            Err(error) => {
                                self.error = Some(error.clone());
                                self.packages.clear();
                            }
                        }
                        self.fetch_promise = None;
                        self.is_loading = false;
                    } else {
                        ctx.request_repaint();
                    }
                }

                // Handle install promise
                if let Some(promise) = &self.install_promise {
                    if let Some(result) = promise.ready() {
                        match result {
                            Ok(install_result) => {
                                self.install_result = Some(install_result.clone());
                                self.error = None;
                                // Refresh the package list after installation
                                // self.fetch_updates_async();
                            }
                            Err(error) => {
                                self.error = Some(error.clone());
                                self.install_result = None;
                            }
                        }
                        self.install_promise = None;
                        self.updating_package_id = None;
                        self.cancel_token = None;
                        self.cancel_clicked = false;
                    } else {
                        ctx.request_repaint();
                    }
                }

                // Display install result
                if let Some(install_result) = &self.install_result.clone() {
                    let is_success = install_result.status == "Ok";
                    let is_error = install_result.status == "InstallError";
                    
                    Frame::new()
                        .fill(if is_success {
                            Color32::from_rgb(34, 197, 94)  // Green for success
                        } else if is_error {
                            Color32::from_rgb(220, 38, 38)  // Red for install error
                        } else {
                            Color32::from_rgb(234, 179, 8)  // Yellow for other statuses
                        })
                        .corner_radius(8.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    let status_text = if is_success {
                                        format!("Installation de {} réussie", install_result.name)
                                    } else if is_error {
                                        format!("Erreur lors de l'installation de {} (Code: {})", 
                                            install_result.name, install_result.installer_error_code)
                                    } else {
                                        format!("{} - Status: {}", install_result.name, install_result.status)
                                    };
                                    
                                    ui.colored_label(Color32::WHITE, status_text);
                                    
                                    // Debug info
                                    ui.label(
                                        RichText::new(format!("ID: {} | Source: {} | Reboot: {}", 
                                            install_result.id, 
                                            install_result.source,
                                            if install_result.reboot_required { "Oui" } else { "Non" }
                                        ))
                                        .size(12.0)
                                        .color(Color32::from_rgb(220, 220, 220))
                                    );
                                });
                                
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.small_button("×").clicked() {
                                        self.install_result = None;
                                    }
                                });
                            });
                        });
                    ui.add_space(10.0);
                }
                
                // Display error message
                if let Some(error) = &self.error.clone() {
                    let (color, icon, message) = match error {
                        WingetError::NoUpdatesAvailable => (
                            Color32::from_rgb(59, 130, 246),  // Blue for info
                            "ℹ",
                            "Aucune mise à jour disponible.".to_string()
                        ),
                        WingetError::PowerShellNotFound => (
                            Color32::from_rgb(220, 38, 38),  // Red for critical
                            "⚠",
                            "PowerShell n'est pas installé ou n'est pas accessible.".to_string()
                        ),
                        WingetError::WinGetModuleNotInstalled => (
                            Color32::from_rgb(220, 38, 38),
                            "⚠",
                            "Le module WinGet n'est pas installé. Installez-le avec: Install-Module Microsoft.WinGet.Client".to_string()
                        ),
                        WingetError::ScriptExecutionDisabled => (
                            Color32::from_rgb(220, 38, 38),
                            "⚠",
                            "L'exécution de scripts PowerShell est désactivée. Exécutez PowerShell en tant qu'administrateur et tapez: Set-ExecutionPolicy RemoteSigned".to_string()
                        ),
                        WingetError::PowerShellError { stderr } => (
                            Color32::from_rgb(220, 38, 38),
                            "⚠",
                            format!("Erreur PowerShell: {}", stderr)
                        ),
                        WingetError::JsonParseError { error, raw_data } => (
                            Color32::from_rgb(220, 38, 38),
                            "⚠",
                            format!("Erreur d'analyse JSON: {}\nDonnées brutes: {}", error, raw_data)
                        ),
                        WingetError::CommandFailed { error } => (
                            Color32::from_rgb(220, 38, 38),
                            "⚠",
                            format!("Échec de la commande: {}", error)
                        ),
                        WingetError::Cancelled => (
                            Color32::from_rgb(234, 179, 8),  // Yellow for warning
                            "⏸",
                            "Mise à jour annulée par l'utilisateur.".to_string()
                        ),
                    };
                    
                    Frame::new()
                        .fill(color)
                        .corner_radius(8.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.colored_label(
                                        Color32::WHITE,
                                        format!("{} {}", icon, message)
                                    );
                                });
                                
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.small_button("×").clicked() {
                                        self.error = None;
                                    }
                                });
                            });
                        });
                    ui.add_space(10.0);
                }

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut package_to_update = None;
                        for package in &self.packages {
                            ui.add_space(4.0);
                            Frame::new()
                                .fill(Color32::from_rgb(45, 45, 45))
                                .corner_radius(8.0)
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.heading(
                                                RichText::new(&package.name)
                                                    .size(18.0)
                                                    .color(Color32::from_rgb(200, 200, 200))
                                            );
                                            ui.label(
                                                RichText::new(&package.id)
                                                    .color(Color32::from_rgb(150, 150, 150))
                                            );
                                        });

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            let is_updating_this_package = self.updating_package_id.as_ref() == Some(&package.id);

                                            // Show cancel button if this package is updating
                                            if is_updating_this_package {
                                                let (button_text, button_color) = if self.cancel_clicked {
                                                    ("Forcer ?", Color32::from_rgb(185, 28, 28)) // Darker red for force
                                                } else {
                                                    ("Annuler", Color32::from_rgb(220, 38, 38)) // Regular red for cancel
                                                };

                                                let cancel_button = ui.add_sized(
                                                    [100.0, 30.0],
                                                    Button::new(
                                                        RichText::new(button_text)
                                                            .color(Color32::WHITE)
                                                    ).fill(button_color)
                                                );

                                                if cancel_button.clicked() {
                                                    if let Some(token) = &self.cancel_token {
                                                        if self.cancel_clicked {
                                                            // Second click: force cancel
                                                            token.force_cancel();
                                                        } else {
                                                            // First click: safe cancel
                                                            token.cancel();
                                                            self.cancel_clicked = true;
                                                        }
                                                    }
                                                }
                                            }

                                            if ui.add_sized(
                                                [100.0, 30.0],
                                                Button::new(
                                                    RichText::new("Rechercher")
                                                        .color(Color32::WHITE)
                                                ).fill(Color32::from_rgb(59, 130, 246))
                                            ).clicked() {
                                                let search_url = format!(
                                                    "https://www.google.com/search?q={}+software+download",
                                                    package.name.replace(" ", "+")
                                                );
                                                let _ = open_url(&search_url);
                                            }

                                            let update_button = ui.add_sized(
                                                [100.0, 30.0],
                                                Button::new(
                                                    RichText::new(
                                                        if is_updating_this_package {
                                                            "Mise à jour..."
                                                        } else {
                                                            "Installer"
                                                        }
                                                    ).color(Color32::BLACK)
                                                )
                                                    .fill(if is_updating_this_package {
                                                        Color32::from_rgb(70, 70, 70)
                                                    } else {
                                                        Color32::from_rgb(234, 179, 8)
                                                    })
                                            );

                                            if update_button.clicked() && self.updating_package_id.is_none() {
                                                package_to_update = Some(package.id.clone());
                                            }

                                            if is_updating_this_package {
                                                ui.spinner();
                                            }

                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(&package.available_version)
                                                        .color(if package.version != package.available_version {
                                                            Color32::from_rgb(234, 179, 8)
                                                        } else {
                                                            Color32::from_rgb(34, 197, 94)
                                                        })
                                                        .size(16.0)
                                                );

                                                ui.label(
                                                    RichText::new(" to ")
                                                        .color(Color32::from_rgb(100, 100, 100))
                                                        .size(16.0)
                                                );

                                                ui.label(
                                                    RichText::new(&package.version)
                                                        .color(Color32::from_rgb(150, 150, 150))
                                                        .size(16.0)
                                                );
                                            });

                                        });
                                    });
                                });
                        }

                        if let Some(id) = package_to_update {
                            self.update_single_async(&id);
                        }
                    });
            });
        });
    }
}
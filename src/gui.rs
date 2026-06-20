use crate::config::*;
use crate::winget_manager::{open_url, CancellationToken, InstallResult, Package, WingetError, WingetManager};
use eframe::egui::{vec2, Align, Button, CentralPanel, Color32, Frame, Layout, RichText, ScrollArea, TextEdit};
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
    search_filter: String,
    updating_all: bool,
    packages_to_update_all: Vec<String>,
    reboot_required: bool,
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
            search_filter: String::new(),
            updating_all: false,
            packages_to_update_all: Vec::new(),
            reboot_required: false,
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

    fn start_update_all(&mut self) {
        self.updating_all = true;
        self.packages_to_update_all = self.packages.iter().map(|p| p.id.clone()).collect();
        if !self.packages_to_update_all.is_empty() {
            let next_id = self.packages_to_update_all.remove(0);
            self.update_single_async(&next_id);
        }
    }

    fn get_filtered_packages(&self) -> Vec<&Package> {
        if self.search_filter.is_empty() {
            self.packages.iter().collect()
        } else {
            let filter_lower = self.search_filter.to_lowercase();
            self.packages
                .iter()
                .filter(|p| {
                    p.name.to_lowercase().contains(&filter_lower)
                        || p.id.to_lowercase().contains(&filter_lower)
                })
                .collect()
        }
    }
}

impl eframe::App for PackageApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.global_style()).clone();
        style.spacing.item_spacing = vec2(ITEM_SPACING_H, ITEM_SPACING_V);
        style.visuals.resize_corner_size = CORNER_RADIUS_LARGE.into();
        style.visuals.code_bg_color = COLOR_BG_CODE;
        style.visuals.window_fill = COLOR_BG_PRIMARY;
        ctx.set_global_style(style);

        CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let title_text = format!(
                        "Gestionnaire de Mises à Jour (winget) - {} mise(s) à jour",
                        self.packages.len()
                    );
                    ui.heading(
                        RichText::new(&title_text)
                            .size(FONT_SIZE_HEADING)
                            .color(COLOR_TEXT_PRIMARY)
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let refresh_button = ui.add_sized(
                            [BUTTON_LARGE_WIDTH, BUTTON_HEIGHT],
                            Button::new(
                                RichText::new(
                                    if self.is_loading {
                                        "⟳ Chargement..."
                                    } else {
                                        "⟳ Rafraîchir"
                                    }
                                )
                                .size(FONT_SIZE_NORMAL)
                                .color(COLOR_TEXT_WHITE)
                            )
                            .fill(if self.is_loading {
                                COLOR_BTN_DISABLED
                            } else {
                                COLOR_BTN_PRIMARY
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

                ui.add_space(SPACING_LARGE);

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
                                if install_result.reboot_required {
                                    self.reboot_required = true;
                                }
                                self.error = None;
                                // Auto-refresh after installation
                                if self.updating_all && !self.packages_to_update_all.is_empty() {
                                    let next_id = self.packages_to_update_all.remove(0);
                                    self.update_single_async(&next_id);
                                } else if self.updating_all {
                                    self.updating_all = false;
                                    // Refresh package list after all updates
                                    self.fetch_updates_async();
                                } else {
                                    // Refresh for single update
                                    self.fetch_updates_async();
                                }
                            }
                            Err(error) => {
                                self.error = Some(error.clone());
                                self.install_result = None;
                                if self.updating_all && !self.packages_to_update_all.is_empty() {
                                    let next_id = self.packages_to_update_all.remove(0);
                                    self.update_single_async(&next_id);
                                } else {
                                    self.updating_all = false;
                                }
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

                // Display reboot required notification
                if self.reboot_required {
                    Frame::new()
                        .fill(COLOR_WARNING)
                        .corner_radius(CORNER_RADIUS)
                        .inner_margin(INNER_MARGIN_MESSAGE)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.colored_label(
                                        COLOR_TEXT_WHITE,
                                        "🔄 Un redémarrage est nécessaire pour appliquer les mises à jour."
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.small_button("×").clicked() {
                                        self.reboot_required = false;
                                    }
                                });
                            });
                        });
                    ui.add_space(SPACING_LARGE);
                }

                // Display install result
                if let Some(install_result) = &self.install_result.clone() {
                    let is_success = install_result.status == "Ok";
                    let is_error = install_result.status == "InstallError";
                    
                    Frame::new()
                        .fill(if is_success {
                            COLOR_SUCCESS
                        } else if is_error {
                            COLOR_ERROR
                        } else {
                            COLOR_WARNING
                        })
                        .corner_radius(CORNER_RADIUS)
                        .inner_margin(INNER_MARGIN_MESSAGE)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    let status_text = if is_success {
                                        format!("✓ Installation de {} réussie", install_result.name)
                                    } else if is_error {
                                        format!("✗ Erreur lors de l'installation de {} (Code: {})", 
                                            install_result.name, install_result.installer_error_code)
                                    } else {
                                        format!("⚙ {} - Status: {}", install_result.name, install_result.status)
                                    };
                                    
                                    ui.colored_label(COLOR_TEXT_WHITE, status_text);
                                    
                                    ui.label(
                                        RichText::new(format!("Source: {} | Reboot: {}", 
                                            install_result.source,
                                            if install_result.reboot_required { "Oui" } else { "Non" }
                                        ))
                                        .size(FONT_SIZE_SMALL)
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
                    ui.add_space(SPACING_LARGE);
                }
                
                // Display error message
                if let Some(error) = &self.error.clone() {
                    let (color, icon, message) = match error {
                        WingetError::NoUpdatesAvailable => (
                            COLOR_INFO,
                            "ℹ",
                            "Aucune mise à jour disponible.".to_string()
                        ),
                        WingetError::PowerShellNotFound => (
                            COLOR_ERROR,
                            "⚠",
                            "PowerShell n'est pas installé ou n'est pas accessible.".to_string()
                        ),
                        WingetError::WinGetModuleNotInstalled => (
                            COLOR_ERROR,
                            "⚠",
                            "Le module WinGet n'est pas installé. Installez-le avec: Install-Module Microsoft.WinGet.Client".to_string()
                        ),
                        WingetError::ScriptExecutionDisabled => (
                            COLOR_ERROR,
                            "⚠",
                            "L'exécution de scripts PowerShell est désactivée. Exécutez PowerShell en tant qu'administrateur et tapez: Set-ExecutionPolicy RemoteSigned".to_string()
                        ),
                        WingetError::PowerShellError { stderr } => (
                            COLOR_ERROR,
                            "⚠",
                            format!("Erreur PowerShell: {}", stderr)
                        ),
                        WingetError::JsonParseError { error, raw_data } => (
                            COLOR_ERROR,
                            "⚠",
                            format!("Erreur d'analyse JSON: {}\nDonnées brutes: {}", error, raw_data)
                        ),
                        WingetError::CommandFailed { error } => (
                            COLOR_ERROR,
                            "⚠",
                            format!("Échec de la commande: {}", error)
                        ),
                        WingetError::Cancelled => (
                            COLOR_WARNING,
                            "⏸",
                            "Mise à jour annulée par l'utilisateur.".to_string()
                        ),
                    };
                    
                    Frame::new()
                        .fill(color)
                        .corner_radius(CORNER_RADIUS)
                        .inner_margin(INNER_MARGIN_MESSAGE)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.colored_label(
                                        COLOR_TEXT_WHITE,
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
                    ui.add_space(SPACING_LARGE);
                }

                // Search filter
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍 Rechercher:").size(FONT_SIZE_NORMAL));
                    TextEdit::singleline(&mut self.search_filter)
                        .hint_text("Nom ou ID du paquet...")
                        .desired_width(f32::INFINITY)
                        .show(ui);
                });

                ui.add_space(SPACING_MEDIUM);

                // Control buttons
                ui.horizontal(|ui| {
                    let update_all_enabled = !self.packages.is_empty() 
                        && !self.is_loading 
                        && self.updating_package_id.is_none()
                        && !self.updating_all;
                    
                    if update_all_enabled {
                        let update_all_button = ui.add_sized(
                            [BUTTON_LARGE_WIDTH + 30.0, BUTTON_HEIGHT],
                            Button::new(
                                RichText::new("📦 Tout installer")
                                    .color(COLOR_TEXT_WHITE)
                            )
                            .fill(COLOR_BTN_SECONDARY)
                        );

                        if update_all_button.clicked() {
                            self.start_update_all();
                        }
                    } else {
                        let button_text = if self.updating_all {
                            "⏳ Mise à jour..."
                        } else {
                            "📦 Tout installer"
                        };
                        let _ = ui.add_sized(
                            [BUTTON_LARGE_WIDTH + 30.0, BUTTON_HEIGHT],
                            Button::new(
                                RichText::new(button_text)
                                    .color(COLOR_TEXT_TERTIARY)
                            )
                            .fill(COLOR_BTN_DISABLED)
                        );
                    }

                    ui.label(
                        RichText::new(format!("{} paquet(s) visible(s)", self.get_filtered_packages().len()))
                            .size(FONT_SIZE_NORMAL)
                            .color(COLOR_TEXT_SECONDARY)
                    );
                });

                ui.add_space(SPACING_LARGE);

                // Package list
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut package_to_update = None;
                        let filtered_packages_ids: Vec<_> = self.get_filtered_packages()
                            .iter()
                            .map(|p| (p.id.clone(), p.name.clone(), p.version.clone(), p.available_version.clone()))
                            .collect();
                        
                        for (package_id, package_name, version, available_version) in filtered_packages_ids {
                            ui.add_space(SPACING_SMALL);
                            Frame::new()
                                .fill(COLOR_BG_SECONDARY)
                                .corner_radius(CORNER_RADIUS)
                                .inner_margin(INNER_MARGIN)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.heading(
                                                RichText::new(&package_name)
                                                    .size(FONT_SIZE_TITLE)
                                                    .color(COLOR_TEXT_PRIMARY)
                                            );
                                            ui.label(
                                                RichText::new(&package_id)
                                                    .color(COLOR_TEXT_SECONDARY)
                                            );
                                        });

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            let is_updating_this_package = self.updating_package_id.as_ref() == Some(&package_id);

                                            // Show cancel button if this package is updating
                                            if is_updating_this_package {
                                                let (button_text, button_color) = if self.cancel_clicked {
                                                    ("Forcer ?", COLOR_CRITICAL_BUTTON)
                                                } else {
                                                    ("Annuler", COLOR_BTN_DANGER)
                                                };

                                                let cancel_button = ui.add_sized(
                                                    [BUTTON_WIDTH, BUTTON_HEIGHT],
                                                    Button::new(
                                                        RichText::new(button_text)
                                                            .color(COLOR_TEXT_WHITE)
                                                    ).fill(button_color)
                                                );

                                                if cancel_button.clicked() {
                                                    if let Some(token) = &self.cancel_token {
                                                        if self.cancel_clicked {
                                                            token.force_cancel();
                                                        } else {
                                                            token.cancel();
                                                            self.cancel_clicked = true;
                                                        }
                                                    }
                                                }
                                            }

                                            if ui.add_sized(
                                                [BUTTON_WIDTH, BUTTON_HEIGHT],
                                                Button::new(
                                                    RichText::new("🔗 Info")
                                                        .color(COLOR_TEXT_WHITE)
                                                ).fill(COLOR_BTN_PRIMARY)
                                            ).clicked() {
                                                let search_url = format!(
                                                    "https://www.google.com/search?q={}+software+download",
                                                    package_name.replace(" ", "+")
                                                );
                                                let _ = open_url(&search_url);
                                            }

                                            let update_button = ui.add_sized(
                                                [BUTTON_WIDTH, BUTTON_HEIGHT],
                                                Button::new(
                                                    RichText::new(
                                                        if is_updating_this_package {
                                                            "⏳..."
                                                        } else {
                                                            "⬆ Maj"
                                                        }
                                                    ).color(COLOR_TEXT_WHITE)
                                                )
                                                    .fill(if is_updating_this_package {
                                                        COLOR_BTN_DISABLED
                                                    } else {
                                                        COLOR_BTN_SECONDARY
                                                    })
                                            );

                                            if update_button.clicked() && self.updating_package_id.is_none() && !self.updating_all {
                                                package_to_update = Some(package_id.clone());
                                            }

                                            if is_updating_this_package {
                                                ui.spinner();
                                            }

                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(&available_version)
                                                        .color(COLOR_WARNING)
                                                        .size(FONT_SIZE_NORMAL)
                                                );

                                                ui.label(
                                                    RichText::new("→")
                                                        .color(COLOR_TEXT_TERTIARY)
                                                        .size(FONT_SIZE_NORMAL)
                                                );

                                                ui.label(
                                                    RichText::new(&version)
                                                        .color(COLOR_TEXT_SECONDARY)
                                                        .size(FONT_SIZE_NORMAL)
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

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // This method is deprecated but still required by the App trait in some versions
        // The main UI logic is in the update method above
    }
}
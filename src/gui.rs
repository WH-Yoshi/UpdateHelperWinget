use crate::winget_manager::{open_url, Package, WingetManager};
use eframe::egui;
use poll_promise::Promise;

/// Structure principale de l'application GUI pour gérer les packages winget
/// Contient l'état de l'interface utilisateur et les données des packages
pub struct PackageApp {
    /// Liste des packages disponibles pour mise à jour
    packages: Vec<Package>,
    /// Message d'erreur à afficher à l'utilisateur
    error_message: String,
    /// Promise pour les opérations asynchrones (chargement ou mise à jour)
    promise: Option<Promise<Result<Vec<Package>, String>>>,
    /// Indicateur de chargement en cours
    is_loading: bool,
    /// ID du package en cours de mise à jour (pour afficher le spinner)
    updating_package_id: Option<String>,
}

/// Implémentation du trait Default pour initialiser l'application avec des valeurs par défaut
impl Default for PackageApp {
    fn default() -> Self {
        Self {
            packages: Vec::new(),
            error_message: String::new(),
            promise: None,
            is_loading: false,
            updating_package_id: None,
        }
    }
}

impl PackageApp {
    /// Constructeur de l'application, initialise et lance la récupération des mises à jour
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.fetch_updates_async();
        app
    }

    /// Lance la récupération asynchrone des mises à jour disponibles via winget
    /// Évite les appels multiples si une opération est déjà en cours
    fn fetch_updates_async(&mut self) {
        // Vérifie qu'aucune opération n'est déjà en cours
        if self.promise.is_some() || self.is_loading {
            return;
        }

        // Active l'indicateur de chargement
        self.is_loading = true;

        // Lance la récupération des mises à jour dans un thread séparé
        let promise = Promise::spawn_thread("winget_fetch", || {
            let rx = WingetManager::fetch_updates_async();
            rx.recv().unwrap_or(Err("Erreur de communication avec le thread".to_string()))
        });

        self.promise = Some(promise);
    }

    /// Lance la mise à jour asynchrone d'un package spécifique
    /// Évite les mises à jour multiples simultanées
    fn update_single_async(&mut self, package_id: &str) {
        // Vérifie qu'aucune opération n'est en cours
        if self.promise.is_some() || self.updating_package_id.is_some() {
            return;
        }

        // Marque ce package comme étant en cours de mise à jour
        self.updating_package_id = Some(package_id.to_string());

        // Clone l'ID pour l'utiliser dans la closure
        let package_id = package_id.to_string();
        // Lance l'installation dans un thread séparé
        let promise = Promise::spawn_thread("winget_install_single", move || {
            let rx = WingetManager::install_single(&package_id);
            rx.recv().unwrap_or(Err("Erreur de communication avec le thread".to_string()))
        });

        self.promise = Some(promise);
    }
}

/// Implémentation du trait eframe::App pour définir le comportement de l'interface utilisateur
impl eframe::App for PackageApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Configuration du style visuel de l'interface (thème sombre)
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.visuals.resize_corner_size = 10.0.into();
        style.visuals.code_bg_color = egui::Color32::from_rgb(45, 45, 45);
        style.visuals.window_fill = egui::Color32::from_rgb(32, 32, 32);
        ctx.set_style(style);

        // Panel principal de l'interface utilisateur
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // En-tête avec titre et bouton de rafraîchissement
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Gestionnaire de Mises à Jour (winget)")
                        .size(24.0)
                        .color(egui::Color32::from_rgb(200, 200, 200)));

                    // Alignement à droite pour le bouton de rafraîchissement
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

                        // Gestion du clic sur le bouton de rafraîchissement
                        if refresh_button.clicked() && !self.is_loading {
                            self.fetch_updates_async();
                        }

                        // Affichage du spinner pendant le chargement
                        if self.is_loading {
                            ui.spinner();
                        }
                    });
                });

                ui.add_space(10.0);

                // Gestion des résultats des opérations asynchrones (chargement ou mise à jour)
                if let Some(promise) = &self.promise {
                    if let Some(result) = promise.ready() {
                        match result {
                            Ok(packages) => {
                                // Succès : met à jour la liste des packages et efface les erreurs
                                self.packages = packages.clone();
                                self.error_message.clear();
                            }
                            Err(error) => {
                                // Erreur : affiche le message d'erreur et vide la liste
                                self.error_message = error.clone();
                                self.packages.clear();
                            }
                        }
                        // Réinitialise l'état après traitement du résultat
                        self.promise = None;
                        self.is_loading = false;
                        self.updating_package_id = None;
                    } else {
                        // Demande un nouveau rendu si l'opération n'est pas terminée
                        ctx.request_repaint();
                    }
                }

                // Affichage des messages d'erreur dans un cadre rouge
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

                // Zone de défilement pour afficher la liste des packages
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut package_to_update = None;
                        // Itération sur chaque package pour créer l'interface
                        for package in &self.packages {
                            ui.add_space(4.0);
                            // Cadre pour chaque package avec style sombre
                            egui::Frame::new()
                                .fill(egui::Color32::from_rgb(45, 45, 45))
                                .corner_radius(8.0)
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // Section gauche : informations du package
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

                                        // Section droite : boutons et informations de version
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            // Bouton de recherche Google pour le package
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

                                            // Vérification si ce package est en cours de mise à jour
                                            let is_updating_this_package = self.updating_package_id.as_ref() == Some(&package.id);

                                            // Bouton d'installation/mise à jour
                                            let update_button = ui.add_sized(
                                                [100.0, 30.0],
                                                egui::Button::new(
                                                    egui::RichText::new(
                                                        if is_updating_this_package {
                                                            "Mise à jour..."
                                                        } else {
                                                            "Installer"
                                                        }
                                                    ).color(egui::Color32::BLACK)
                                                )
                                                    .fill(if is_updating_this_package {
                                                        egui::Color32::from_rgb(70, 70, 70)
                                                    } else {
                                                        egui::Color32::from_rgb(234, 179, 8)
                                                    })
                                            );

                                            // Gestion du clic sur le bouton de mise à jour
                                            if update_button.clicked() && self.updating_package_id.is_none() {
                                                package_to_update = Some(package.id.clone());
                                            }

                                            // Affichage du spinner si ce package est en cours de mise à jour
                                            if is_updating_this_package {
                                                ui.spinner();
                                            }

                                            // Affichage des versions (actuelle vers disponible)
                                            ui.horizontal(|ui| {  // Right to Left
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

                        // Lance la mise à jour du package sélectionné (si il y en a un)
                        if let Some(id) = package_to_update {
                            self.update_single_async(&id);
                        }
                    });
            });
        });
    }
}
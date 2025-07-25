// Imports spécifiques à Windows pour gérer les flags de création de processus
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

/// Structure représentant un package winget avec ses informations de version
/// Utilisée pour stocker les détails des packages disponibles pour mise à jour
#[derive(Debug, Clone)]
pub struct Package {
    /// Nom affiché du package (ex: "Visual Studio Code")
    pub name: String,
    /// Identifiant unique du package (ex: "Microsoft.VisualStudioCode")
    pub id: String,
    /// Version actuellement installée
    pub version: String,
    /// Version disponible pour mise à jour
    pub available_version: String,
}

/// Gestionnaire principal pour les opérations winget
/// Fournit des méthodes pour récupérer et installer les mises à jour
pub struct WingetManager;

impl WingetManager {
    /// Récupère de manière asynchrone la liste des packages disponibles pour mise à jour
    /// Utilise la commande 'winget upgrade' et parse la sortie pour extraire les informations
    /// Retourne un receiver qui contiendra le résultat de l'opération
    pub fn fetch_updates_async() -> mpsc::Receiver<Result<Vec<Package>, String>> {
        // Création du canal de communication entre threads
        let (tx, rx) = mpsc::channel();

        // Lance l'opération dans un thread séparé pour éviter de bloquer l'interface
        thread::spawn(move || {
            // Préparation de la commande winget
            let mut command = Command::new("winget");

            #[cfg(target_os = "windows")]
            {
                command.creation_flags(0x08000000);
            }

            // Exécution de la commande 'winget upgrade' pour lister les mises à jour
            let output = command
                .args(["upgrade"])
                .output();


            // TODO: Il faudra check en direct si la ligne suivante s'affiche : 
            // "Do you agree to all the source agreements terms?"
            // Si oui, ouvrir une fenêtre de dialogue pour accepter les termes
            // Sinon, continuer normalement.
            // openAgreementsDialog();

            match output {
                Ok(output) => {
                    // Vérification que la commande s'est exécutée avec succès
                    if output.status.success() {
                        // Tentative de conversion de la sortie en texte UTF-8
                        if let Ok(text) = String::from_utf8(output.stdout) {
                            let mut packages = Vec::new();
                            let lines: Vec<&str> = text.lines().collect();

                            // Vérification qu'il y a suffisamment de lignes (en-tête + données)
                            if lines.len() > 2 {
                                // Parse chaque ligne à partir de la 3ème (ignore l'en-tête)
                                for line in lines[2..].iter() {
                                    // Ignore les lignes vides et les séparateurs
                                    if line.trim().is_empty() || line.starts_with('-') {
                                        continue;
                                    } else if line.ends_with("disponibles.") {
                                        // Fin de la liste des packages
                                        break;
                                    }

                                    // Découpage de la ligne en colonnes
                                    let columns: Vec<&str> = line.split_whitespace().collect();

                                    // Vérification qu'il y a au moins 4 colonnes (nom, id, version, version_disponible)
                                    if columns.len() >= 4 {
                                        let len = columns.len();
                                        // Extraction des informations depuis la fin de la ligne
                                        let available_version = columns[len - 2];
                                        let version = columns[len - 3];
                                        let id = columns[len - 4];
                                        // Le nom peut contenir des espaces, donc on rejoint toutes les colonnes restantes
                                        let name = columns[..(len - 4)].join(" ");

                                        // Création d'un nouveau package avec les informations extraites
                                        packages.push(Package {
                                            name,
                                            id: id.to_string(),
                                            version: version.to_string(),
                                            available_version: available_version.to_string(),
                                        });
                                    }
                                }
                                // Envoi de la liste des packages trouvés
                                tx.send(Ok(packages)).ok();
                            } else {
                                // Aucun package trouvé (liste vide)
                                tx.send(Ok(Vec::new())).ok();
                            }
                        } else {
                            // Erreur de décodage UTF-8 de la sortie winget
                            tx.send(Err("Erreur de décodage UTF-8".to_string())).ok();
                        }
                    } else {
                        // La commande winget a échoué, récupération du message d'erreur
                        let error = String::from_utf8_lossy(&output.stderr).to_string();
                        tx.send(Err(error)).ok();
                    }
                }
                Err(e) => {
                    // Erreur lors du lancement de la commande winget
                    tx.send(Err(format!("Erreur lors de l'exécution de winget: {}", e))).ok();
                }
            }
        });

        // Retour du receiver pour récupérer le résultat
        rx
    }

    /// Installe ou met à jour un package spécifique via son ID
    /// Après l'installation, récupère la liste mise à jour des packages
    /// Retourne un receiver contenant la nouvelle liste ou une erreur
    pub fn install_single(id: &str) -> mpsc::Receiver<Result<Vec<Package>, String>> {
        // Création du canal de communication
        let (tx, rx) = mpsc::channel();
        // Clone de l'ID pour l'utiliser dans le thread
        let id = id.to_string();

        // Lance l'installation dans un thread séparé
        thread::spawn(move || {
            // Préparation de la commande winget
            let mut command = Command::new("winget");

            // Configuration spécifique à Windows pour masquer la console
            #[cfg(target_os = "windows")]
            {
                // Flag CREATE_NO_WINDOW pour éviter l'ouverture d'une fenêtre console
                command.creation_flags(0x08000000);
            }

            // Exécution de la commande d'installation avec acceptation automatique des accords
            let result = command
                .args(["upgrade", "--id", &id, "--accept-source-agreements"])
                .output();

            // Traitement du résultat de l'installation
            match result {
                Ok(output) => {
                    // Vérification que l'installation s'est bien déroulée
                    if output.status.success() {
                        // Récupération de la liste mise à jour des packages après installation
                        let rx_updates = WingetManager::fetch_updates_async();
                        match rx_updates.recv() {
                            Ok(result) => tx.send(result).ok(),
                            Err(_) => tx.send(Err("Erreur lors de la récupération des mises à jour".to_string())).ok(),
                        };
                    } else {
                        // L'installation a échoué, récupération du message d'erreur
                        let error = String::from_utf8_lossy(&output.stderr);
                        tx.send(Err(format!(
                            "Erreur lors de la mise à jour de {id}: {error}"
                        ))).ok();
                    }
                }
                Err(e) => {
                    // Erreur lors du lancement de la commande winget
                    tx.send(Err(format!(
                        "Impossible d'exécuter winget pour {id}: {e}"
                    ))).ok();
                }
            }
        });

        // Retour du receiver pour récupérer le résultat
        rx
    }
}

/// Ouvre une URL dans le navigateur par défaut du système
/// Utilise la commande système appropriée selon l'OS
pub fn open_url(url: &str) {
    // Implémentation spécifique à Windows
    #[cfg(target_os = "windows")]
    {
        // Utilise la commande 'start' de Windows pour ouvrir l'URL dans le navigateur par défaut
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .ok();
    }
}
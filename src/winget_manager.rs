use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub id: String,
    pub version: String,
    pub available_version: String,
}

pub struct WingetManager;

impl WingetManager {
    pub fn fetch_updates_async() -> mpsc::Receiver<Result<Vec<Package>, String>> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let output = Command::new("winget")
                .creation_flags(0x08000000)
                .args(["upgrade"])  // "--include-unknown"
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
                            } else {
                                tx.send(Ok(Vec::new())).ok();
                            }
                        } else {
                            tx.send(Err("Erreur de décodage UTF-8".to_string())).ok();
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

        rx
    }

    pub fn install_single(id: &str) -> mpsc::Receiver<Result<Vec<Package>, String>> {
        let (tx, rx) = mpsc::channel();
        let id = id.to_string();

        thread::spawn(move || {
            let result = Command::new("winget")
                .creation_flags(0x08000000)
                .args(["upgrade", "--id", &id, "--accept-source-agreements"])
                .output();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        let rx_updates = WingetManager::fetch_updates_async();
                        match rx_updates.recv() {
                            Ok(result) => tx.send(result).ok(),
                            Err(_) => tx.send(Err("Erreur lors de la récupération des mises à jour".to_string())).ok(),
                        };
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        tx.send(Err(format!(
                            "Erreur lors de la mise à jour de {id}: {error}"
                        ))).ok();
                    }
                }
                Err(e) => {
                    tx.send(Err(format!(
                        "Impossible d'exécuter winget pour {id}: {e}"
                    ))).ok();
                }
            }
        });

        rx
    }
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .ok();
    }
}
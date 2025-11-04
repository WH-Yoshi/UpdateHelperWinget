use serde_json::from_str;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

#[derive(Debug, Clone, serde::Deserialize)]
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
            let ps_script = r#"
                Import-Module Microsoft.WinGet.Client
                $updates = Get-WinGetPackage | Where-Object { $_.IsUpdateAvailable }
                $updates | Select-Object Name, Id, InstalledVersion, @{Name='AvailableVersion'; Expression={ $_.AvailableVersions[0] }} | ConvertTo-Json -Compress
            "#;

            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy", "Bypass",
                    "-Command",
                    ps_script,
                ])
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let json = String::from_utf8_lossy(&out.stdout);
                    match from_str::<Vec<Package>>(&json) {
                        Ok(packages) => tx.send(Ok(packages)).ok(),
                        Err(e) => tx.send(Err(format!("JSON parse error: {}", e))).ok(),
                    };
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    tx.send(Err(err.to_string())).ok();
                }
                Err(e) => { tx.send(Err(format!("Failed to run PowerShell: {}", e))).ok(); }
            }
        });

        rx
    }

    pub fn install_single(id: &str) -> mpsc::Receiver<Result<Vec<Package>, String>> {
        let (tx, rx) = mpsc::channel();
        let id = id.to_string();

        thread::spawn(move || {
            let ps_script = format!(r#"
                Import-Module Microsoft.WinGet.Client
                Install-WinGetPackage -id '{id}' -AcceptSourceAgreements | Out-Null
                $updates = Get-WinGetPackage | Where-Object {{ $_.IsUpdateAvailable }}
                $updates | Select-Object name, id, version, available_version | ConvertTo-Json -Compress
            "#);

            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy", "Bypass",
                    "-Command",
                    &ps_script,
                ])
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let json = String::from_utf8_lossy(&out.stdout);
                    match from_str::<Vec<Package>>(&json) {
                        Ok(packages) => tx.send(Ok(packages)).ok(),
                        Err(e) => tx.send(Err(format!("JSON parse error: {}", e))).ok(),
                    };
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    tx.send(Err(err.to_string())).ok();
                }
                Err(e) => { tx.send(Err(format!("Failed to run PowerShell: {}", e))).ok(); }
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
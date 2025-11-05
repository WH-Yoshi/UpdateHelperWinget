use serde_json::from_str;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use open;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Package {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "InstalledVersion")]
    pub version: String,
    #[serde(rename = "AvailableVersion")]
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
                $updates | Select-Object Name, Id, InstalledVersion, @{Name='AvailableVersion'; Expression={ $_.AvailableVersions[0] }} | ConvertTo-Json -Compress -AsArray
            "#;

            let output = Command::new("pwsh")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy", "Bypass",
                    "-Command",
                    ps_script,
                ])
                .creation_flags(0x08000000)
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
                $updates | Select-Object Name, Id, InstalledVersion, @{{Name='AvailableVersion'; Expression={{ $_.AvailableVersions[0] }}}} | ConvertTo-Json -AsArray
            "#);

            let output = Command::new("pwsh")
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

pub fn open_url(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    open::that(url)?;
    Ok(())
}
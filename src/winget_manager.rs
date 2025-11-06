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
    #[serde(rename = "InstalledVersion", default)]
    pub version: String,
    #[serde(rename = "AvailableVersion", default)]
    pub available_version: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InstallResult {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Source", default)]
    pub source: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "RebootRequired", default)]
    pub reboot_required: bool,
    #[serde(rename = "InstallerErrorCode", default)]
    pub installer_error_code: i32,
}

#[derive(Debug, Clone)]
pub enum WingetError {
    NoUpdatesAvailable,
    PowerShellNotFound,
    WinGetModuleNotInstalled,
    ScriptExecutionDisabled,
    PowerShellError { stderr: String },
    JsonParseError { error: String, raw_data: String },
    CommandFailed { error: String },
}

impl WingetError {
    pub fn from_stderr(stderr: &str) -> Self {
        if stderr.contains("not found") || stderr.contains("not recognized") {
            WingetError::PowerShellNotFound
        } else if stderr.contains("execution of scripts is disabled") {
            WingetError::ScriptExecutionDisabled
        } else if stderr.contains("module") && (stderr.contains("not") || stderr.contains("cannot")) {
            WingetError::WinGetModuleNotInstalled
        } else {
            WingetError::PowerShellError { 
                stderr: stderr.trim().to_string() 
            }
        }
    }
}

pub struct WingetManager;

impl WingetManager {
    pub fn fetch_updates_async() -> mpsc::Receiver<Result<Vec<Package>, WingetError>> {
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
                    let json = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if json.is_empty() || json == "null" {
                        let _ = tx.send(Err(WingetError::NoUpdatesAvailable));
                    } else {
                        match from_str::<Vec<Package>>(&json) {
                            Ok(packages) if packages.is_empty() => {
                                let _ = tx.send(Err(WingetError::NoUpdatesAvailable));
                            },
                            Ok(packages) => { 
                                let _ = tx.send(Ok(packages)); 
                            },
                            Err(e) => { 
                                let _ = tx.send(Err(WingetError::JsonParseError {
                                    error: e.to_string(),
                                    raw_data: json,
                                })); 
                            },
                        }
                    }
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    let _ = tx.send(Err(WingetError::from_stderr(&err)));
                }
                Err(e) => { 
                    let _ = tx.send(Err(WingetError::CommandFailed { 
                        error: e.to_string() 
                    })); 
                }
            }
        });

        rx
    }

    pub fn install_single(id: &str) -> mpsc::Receiver<Result<InstallResult, WingetError>> {
        let (tx, rx) = mpsc::channel();
        let id = id.to_string();

        thread::spawn(move || {
            let ps_script = format!(r#"
                Import-Module Microsoft.WinGet.Client
                Install-WinGetPackage -id '{id}' | ConvertTo-Json -Compress -Depth 10
            "#);

            let output = Command::new("pwsh")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy", "Bypass",
                    "-Command",
                    &ps_script,
                ])
                .creation_flags(0x08000000)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let json = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    match from_str::<InstallResult>(&json) {
                        Ok(result) => {
                            tx.send(Ok(result)).ok();
                        },
                        Err(e) => {
                            tx.send(Err(WingetError::JsonParseError {
                                error: e.to_string(),
                                raw_data: json,
                            })).ok();
                        },
                    }
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    tx.send(Err(WingetError::from_stderr(&err))).ok();
                }
                Err(e) => { 
                    tx.send(Err(WingetError::CommandFailed { 
                        error: e.to_string() 
                    })).ok(); 
                }
            }
        });

        rx
    }
}

pub fn open_url(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    open::that(url)?;
    Ok(())
}
use serde_json::from_str;
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
#[cfg(target_os = "windows")]
use std::{os::windows::process::CommandExt, process::Command};
use open;

/// Token to allow cancelling an ongoing operation
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    force_cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            force_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn force_cancel(&self) {
        self.force_cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn is_force_cancelled(&self) -> bool {
        self.force_cancelled.load(Ordering::SeqCst)
    }
}

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
    Cancelled,
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
    /// Escape single quotes in PowerShell strings to prevent command injection
    fn escape_powershell_string(s: &str) -> String {
        s.replace("'", "''")
    }

    fn execute_powershell_async<T>(
        script: String,
        parser: fn(String) -> Result<T, WingetError>,
    ) -> mpsc::Receiver<Result<T, WingetError>>
    where
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = std::panic::catch_unwind(|| {
                #[cfg(target_os = "windows")]
                let output = Command::new("pwsh")
                    .args([
                        "-NoProfile",
                        "-ExecutionPolicy", "Bypass",
                        "-Command",
                        &script,
                    ])
                    .creation_flags(0x08000000)
                    .output();

                #[cfg(not(target_os = "windows"))]
                let output: Result<std::process::Output, std::io::Error> = 
                    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Not Windows"));

                match output {
                    Ok(out) if out.status.success() => {
                        let json = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        parser(json)
                    }
                    Ok(out) => {
                        let err = String::from_utf8_lossy(&out.stderr);
                        Err(WingetError::from_stderr(&err))
                    }
                    Err(e) => Err(WingetError::CommandFailed {
                        error: e.to_string(),
                    }),
                }
            });

            let final_result = result.unwrap_or_else(|_| Err(WingetError::CommandFailed {
                error: "Parser panicked during execution".to_string(),
            }));

            let _ = tx.send(final_result);
        });

        rx
    }

    pub fn fetch_updates_async() -> mpsc::Receiver<Result<Vec<Package>, WingetError>> {
        let script = r#"
            Import-Module Microsoft.WinGet.Client
            $updates = Get-WinGetPackage | Where-Object { $_.IsUpdateAvailable }
            $updates | Select-Object Name, Id, InstalledVersion, @{Name='AvailableVersion'; Expression={ $_.AvailableVersions[0] }} | ConvertTo-Json -Compress -AsArray
        "#.to_string();

        Self::execute_powershell_async(script, |json| {
            if json.is_empty() || json == "null" {
                return Err(WingetError::NoUpdatesAvailable);
            }
            match from_str::<Vec<Package>>(&json) {
                Ok(packages) if packages.is_empty() => Err(WingetError::NoUpdatesAvailable),
                Ok(packages) => Ok(packages),
                Err(e) => Err(WingetError::JsonParseError {
                    error: e.to_string(),
                    raw_data: json,
                }),
            }
        })
    }

    pub fn install_single(id: &str, cancel_token: CancellationToken) -> mpsc::Receiver<Result<InstallResult, WingetError>> {
        let escaped_id = Self::escape_powershell_string(id);
        let script = format!(r#"
            Import-Module Microsoft.WinGet.Client
            Install-WinGetPackage -id '{escaped_id}' | ConvertTo-Json -Compress -Depth 10
        "#);

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            if cancel_token.is_cancelled() {
                let _ = tx.send(Err(WingetError::Cancelled));
                return;
            }

            let result = std::panic::catch_unwind(|| {
                #[cfg(target_os = "windows")]
                {
                    let child_process = Command::new("pwsh")
                        .args([
                            "-NoProfile",
                            "-ExecutionPolicy", "Bypass",
                            "-Command",
                            &script,
                        ])
                        .creation_flags(0x08000000)
                        .spawn();

                    match child_process {
                        #[allow(unused_mut)]
                        Ok(mut child) => {
                            let child_id = child.id();
                            let cancel_token_clone = cancel_token.clone();
                            
                            // Monitor thread to handle force cancellation
                            thread::spawn(move || {
                                while !cancel_token_clone.is_force_cancelled() {
                                    use std::time::Duration;

                                    thread::sleep(Duration::from_millis(100));
                                }
                                // Force kill the process tree
                                let _ = Command::new("taskkill")
                                    .args(["/F", "/T", "/PID", &child_id.to_string()])
                                    .creation_flags(0x08000000)
                                    .output();
                            });

                            // Wait for the process to complete
                            let output = child.wait_with_output();

                            // Check if cancelled
                            if cancel_token.is_cancelled() || cancel_token.is_force_cancelled() {
                                return Err(WingetError::Cancelled);
                            }

                            match output {
                                Ok(out) if out.status.success() => {
                                    let json = String::from_utf8_lossy(&out.stdout).trim().to_string();
                                    from_str::<InstallResult>(&json).map_err(|e| WingetError::JsonParseError {
                                        error: e.to_string(),
                                        raw_data: json,
                                    })
                                }
                                Ok(out) => {
                                    let err = String::from_utf8_lossy(&out.stderr);
                                    Err(WingetError::from_stderr(&err))
                                }
                                Err(e) => Err(WingetError::CommandFailed {
                                    error: e.to_string(),
                                }),
                            }
                        }
                        Err(e) => Err(WingetError::CommandFailed {
                            error: e.to_string(),
                        }),
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    Err(WingetError::CommandFailed {
                        error: "Not supported on non-Windows platforms".to_string(),
                    })
                }
            });

            let final_result = result.unwrap_or_else(|_| Err(WingetError::CommandFailed {
                error: "Parser panicked during execution".to_string(),
            }));

            let _ = tx.send(final_result);
        });

        rx
    }
}



pub fn open_url(url: &str) -> Result<(), String> {
    open::that(url).map_err(|e| format!("Failed to open URL: {}", e))
}
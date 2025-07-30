use crate::winget_manager::{Package, WingetManager};
use poll_promise::Promise;

pub struct PackageApp {
    pub packages: Vec<Package>,
    pub error_message: String,
    pub fetch_promise: Option<Promise<Result<Vec<Package>, String>>>,
    pub update_promise: Option<Promise<Result<Vec<Package>, String>>>,
    pub is_loading: bool,
    pub updating_package_id: Option<String>,
    pub update_queue: Vec<String>,
    pub is_batch_updating: bool,
}

impl Default for PackageApp {
    fn default() -> Self {
        Self {
            packages: Vec::new(),
            error_message: String::new(),
            fetch_promise: None,
            update_promise: None,
            is_loading: false,
            updating_package_id: None,
            update_queue: Vec::new(),
            is_batch_updating: false,
        }
    }
}

impl PackageApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.fetch_updates_async();
        app
    }

    pub fn fetch_updates_async(&mut self) {
        if self.fetch_promise.is_some() || self.is_loading {
            return;
        }

        self.is_loading = true;

        let promise = Promise::spawn_thread("winget_fetch", || {
            let rx = WingetManager::fetch_updates_async();
            rx.recv().unwrap_or(Err("Erreur de communication avec le thread".to_string()))
        });

        self.fetch_promise = Some(promise);
    }

    pub fn update_single_async(&mut self, package_id: &str) {
        if self.updating_package_id.is_some() {
            return;
        }

        self.updating_package_id = Some(package_id.to_string());
        let package_id = package_id.to_string();

        let promise = Promise::spawn_thread("winget_install_single", move || {
            let rx = WingetManager::install_single(&package_id);
            rx.recv().unwrap_or(Err("Erreur de communication avec le thread".to_string()))
        });

        self.update_promise = Some(promise);
    }

    pub fn process_update_queue(&mut self) {
        if self.update_promise.is_some() || self.updating_package_id.is_some() {
            return;
        }

        if let Some(next_package_id) = self.update_queue.first().cloned() {
            self.update_single_async(&next_package_id);
            self.update_queue.remove(0);
        } else {
            self.is_batch_updating = false;
        }
    }

    pub fn add_to_update_queue(&mut self, package_id: String) {
        if !self.update_queue.contains(&package_id) {
            self.update_queue.push(package_id);
        }
    }

    pub fn start_batch_update(&mut self) {
        self.is_batch_updating = true;
        self.process_update_queue();
    }

    pub fn cancel_batch_update(&mut self) {
        self.update_queue.clear();
        self.is_batch_updating = false;
    }

    // Méthodes pour gérer les promesses de fetch/update (traitement des résultats)
    pub fn handle_fetch_promise(&mut self) {
        if let Some(promise) = &self.fetch_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(packages) => {
                        self.packages = packages.clone();
                        self.error_message.clear();
                    }
                    Err(error) => {
                        self.error_message = error.clone();
                        self.packages.clear();
                    }
                }
                self.fetch_promise = None;
                self.is_loading = false;
            }
        }
    }

    pub fn handle_update_promise(&mut self) {
        if let Some(promise) = &self.update_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(packages) => {
                        self.packages = packages.clone();
                        self.error_message.clear();
                    }
                    Err(error) => {
                        self.error_message = error.clone();
                    }
                }
                self.update_promise = None;
                self.updating_package_id = None;
                self.process_update_queue();
            }
        }
    }
}
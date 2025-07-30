pub mod app;
pub mod ui; // Déclarer le nouveau module pour l'UI
// pub mod styles; // Optionnel : pour les styles si ça devient complexe
// pub mod components ; // Optionnel : si vous avez des composants UI réutilisables

pub use app::PackageApp; // Exporter PackageApp pour un accès facile
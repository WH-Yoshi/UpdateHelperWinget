use eframe::egui::Color32;

// Window dimensions
pub const WINDOW_WIDTH: f32 = 1000.0;
pub const WINDOW_HEIGHT: f32 = 700.0;
pub const MIN_WINDOW_WIDTH: f32 = 900.0;
pub const MIN_WINDOW_HEIGHT: f32 = 600.0;

// Colours - Background
pub const COLOR_BG_PRIMARY: Color32 = Color32::from_rgb(32, 32, 32);
pub const COLOR_BG_SECONDARY: Color32 = Color32::from_rgb(45, 45, 45);
pub const COLOR_BG_CODE: Color32 = Color32::from_rgb(45, 45, 45);

// Colours - Text
pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
pub const COLOR_TEXT_SECONDARY: Color32 = Color32::from_rgb(150, 150, 150);
pub const COLOR_TEXT_TERTIARY: Color32 = Color32::from_rgb(100, 100, 100);
pub const COLOR_TEXT_WHITE: Color32 = Color32::WHITE;

// Colours - Status
pub const COLOR_SUCCESS: Color32 = Color32::from_rgb(34, 197, 94);
pub const COLOR_ERROR: Color32 = Color32::from_rgb(220, 38, 38);
pub const COLOR_WARNING: Color32 = Color32::from_rgb(234, 179, 8);
pub const COLOR_INFO: Color32 = Color32::from_rgb(59, 130, 246);
pub const COLOR_CRITICAL_BUTTON: Color32 = Color32::from_rgb(185, 28, 28);

// Colours - Button States
pub const COLOR_BTN_PRIMARY: Color32 = Color32::from_rgb(59, 130, 246);
pub const COLOR_BTN_SECONDARY: Color32 = Color32::from_rgb(234, 179, 8);
pub const COLOR_BTN_DISABLED: Color32 = Color32::from_rgb(70, 70, 70);
pub const COLOR_BTN_DANGER: Color32 = Color32::from_rgb(220, 38, 38);

// Button dimensions
pub const BUTTON_WIDTH: f32 = 100.0;
pub const BUTTON_HEIGHT: f32 = 30.0;
pub const BUTTON_LARGE_WIDTH: f32 = 120.0;

// Spacing
pub const SPACING_LARGE: f32 = 10.0;
pub const SPACING_MEDIUM: f32 = 8.0;
pub const SPACING_SMALL: f32 = 4.0;

// Font sizes
pub const FONT_SIZE_HEADING: f32 = 24.0;
pub const FONT_SIZE_TITLE: f32 = 18.0;
pub const FONT_SIZE_NORMAL: f32 = 16.0;
pub const FONT_SIZE_SMALL: f32 = 12.0;

// Corner radius
pub const CORNER_RADIUS: f32 = 8.0;
pub const CORNER_RADIUS_LARGE: f32 = 10.0;

// Inner margins
pub const INNER_MARGIN: f32 = 12.0;
pub const INNER_MARGIN_MESSAGE: f32 = 8.0;

// Item spacing
pub const ITEM_SPACING_H: f32 = 10.0;
pub const ITEM_SPACING_V: f32 = 10.0;

// Timeouts (in seconds)
pub const INSTALL_TIMEOUT_SECS: u64 = 300; // 5 minutes default
pub const FETCH_TIMEOUT_SECS: u64 = 60; // 1 minute for fetching updates

// UI state
pub const SPINNER_SIZE: f32 = 20.0;

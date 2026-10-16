//! Windows system tray integration and background persistence.

use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

/// Unique identifier for the Equilizer system tray icon.
pub const TRAY_ID: &str = "equilizer-tray";

/// Sets up the Windows system tray icon and attaches user interaction handlers.
pub fn setup_system_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("Failed to load default window icon for system tray")?;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Equilizer - System Audio Equalizer")
        .icon(icon)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                toggle_window_visibility(app);
            }
        })
        .build(app)?;

    Ok(())
}

/// Toggles the visibility of the main workstation window.
/// If visible, minimizes/hides to tray. If hidden, restores and focuses.
pub fn toggle_window_visibility(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(is_visible) = window.is_visible() {
            if is_visible {
                let _ = window.hide();
            } else {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_id_constant() {
        assert_eq!(TRAY_ID, "equilizer-tray");
    }

    #[test]
    fn test_embedded_icon_bytes_valid() {
        let bytes = include_bytes!("../icons/32x32.png");
        assert!(!bytes.is_empty());
        // Verify PNG magic header: \x89PNG\r\n\x1a\n
        assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}

//! Windows system tray integration and background persistence.

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use crate::AppState;

/// Unique identifier for the Equilizer system tray icon.
pub const TRAY_ID: &str = "equilizer-tray";
pub const MENU_OPEN: &str = "tray_open";
pub const MENU_BYPASS: &str = "tray_bypass";
pub const MENU_EXIT: &str = "tray_exit";
pub const PRESET_PREFIX: &str = "preset:";

/// Sets up the Windows system tray icon and attaches user interaction handlers.
pub fn setup_system_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("Failed to load default window icon for system tray")?;

    // Query active settings and presets catalog to build dynamic tray menu
    let (is_bypassed, presets) = if let Some(state) = app.try_state::<AppState>() {
        let bypassed = state
            .storage
            .load_settings()
            .map(|s| s.is_bypassed)
            .unwrap_or(false);
        let list = state.list_presets().unwrap_or_default();
        (bypassed, list)
    } else {
        (false, Vec::new())
    };

    // Construct context menu items
    let open_item = MenuItem::with_id(app, MENU_OPEN, "Open Equilizer", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let bypass_item =
        CheckMenuItem::with_id(app, MENU_BYPASS, "Bypass EQ", true, is_bypassed, None::<&str>)?;

    let presets_sub = Submenu::new(app, "Presets", true)?;
    for preset in presets {
        let menu_id = format!("{}{}", PRESET_PREFIX, preset.id);
        let item = MenuItem::with_id(app, menu_id, &preset.name, true, None::<&str>)?;
        presets_sub.append(&item)?;
    }

    let sep2 = PredefinedMenuItem::separator(app)?;
    let exit_item = MenuItem::with_id(app, MENU_EXIT, "Exit Equilizer", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&open_item, &sep1, &bypass_item, &presets_sub, &sep2, &exit_item],
    )?;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Equilizer - System Audio Equalizer")
        .icon(icon)
        .menu(&menu)
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
        .on_menu_event(move |app, event| {
            let id = event.id.as_ref();
            match id {
                MENU_OPEN => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
                MENU_BYPASS => {
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Ok(settings) = state.storage.load_settings() {
                            let new_bypass = !settings.is_bypassed;
                            if let Ok(()) = state.toggle_bypass(new_bypass) {
                                let _ = app.emit("app:bypass-changed", new_bypass);
                            }
                        }
                    }
                }
                MENU_EXIT => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let _ = state.restore_system_routing();
                    }
                    app.exit(0);
                }
                preset_id if preset_id.starts_with(PRESET_PREFIX) => {
                    let target_id = &preset_id[PRESET_PREFIX.len()..];
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Ok(preset) = state.set_active_preset(target_id) {
                            let _ = app.emit("app:preset-changed", &preset);
                        }
                    }
                }
                _ => {}
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
    fn test_tray_id_and_menu_constants() {
        assert_eq!(TRAY_ID, "equilizer-tray");
        assert_eq!(MENU_OPEN, "tray_open");
        assert_eq!(MENU_BYPASS, "tray_bypass");
        assert_eq!(MENU_EXIT, "tray_exit");
        assert_eq!(PRESET_PREFIX, "preset:");
    }

    #[test]
    fn test_preset_menu_id_formatting() {
        let preset_id = "deep-bass";
        let menu_id = format!("{}{}", PRESET_PREFIX, preset_id);
        assert!(menu_id.starts_with(PRESET_PREFIX));
        assert_eq!(&menu_id[PRESET_PREFIX.len()..], "deep-bass");
    }

    #[test]
    fn test_embedded_icon_bytes_valid() {
        let bytes = include_bytes!("../icons/32x32.png");
        assert!(!bytes.is_empty());
        // Verify PNG magic header: \x89PNG\r\n\x1a\n
        assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}

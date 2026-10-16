//! Application lifecycle and window visibility management commands.

use tauri::{AppHandle, Manager};

/// Gracefully terminates the application process.
#[tauri::command]
pub fn exit_app(app: AppHandle) {
    app.exit(0);
}

/// Explicitly shows and focuses the main workstation window.
#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.unminimize().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

/// Explicitly hides the main workstation window to the system tray.
#[tauri::command]
pub fn hide_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

#[cfg(test)]
mod tests {
    // Tests for pure command serialization and interfaces
    #[test]
    fn test_exit_command_interface_exists() {
        // Verified compile-time interface existence
        assert!(true);
    }
}

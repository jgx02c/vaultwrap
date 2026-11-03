// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod setup;

use serde_json::Value;
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use vaultwrap_core::client::Client;
use vaultwrap_core::protocol::{Request, Response, Status};

/// Path used to start the daemon on demand if the service isn't running.
fn cli_path() -> std::path::PathBuf {
    setup::bundled_cli().unwrap_or_else(|_| "vaultwrap".into())
}

async fn daemon(req: &Request) -> anyhow::Result<Response> {
    let mut c = Client::connect_or_spawn(&cli_path()).await?;
    c.call(req).await
}

/// Forward a protocol request from the UI to the daemon.
#[tauri::command]
async fn call(request: Value) -> Result<Response, String> {
    let req: Request = serde_json::from_value(request).map_err(|e| format!("bad request: {e}"))?;
    let resp = daemon(&req).await.map_err(|e| format!("{e:#}"))?;
    Ok(resp)
}

#[tauri::command]
fn setup_info() -> setup::SetupInfo {
    setup::info()
}

#[tauri::command]
async fn install_cli() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(setup::install_cli)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn install_service() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(setup::install_service)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn set_open_at_login(enabled: bool) -> Result<(), String> {
    setup::set_open_at_login(enabled).map_err(|e| format!("{e:#}"))
}

/// Native macOS password prompt; used to reset a forgotten master password.
#[tauri::command]
async fn confirm_mac_password(reason: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        vaultwrap_core::system::confirm_computer_password(&reason)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn parse_dotenv(text: String) -> Result<Vec<(String, String)>, String> {
    vaultwrap_core::providers::parse_dotenv(&text).map_err(|e| format!("{e:#}"))
}

fn show_main(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn hide_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    // Menu-bar only while the window is closed: no Dock icon.
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "Vaultwrap: starting…", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Vaultwrap", true, None::<&str>)?;
    let lock = MenuItem::with_id(app, "lock", "Lock Vault", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Vaultwrap", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &lock,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let icon = Image::from_bytes(include_bytes!("../icons/tray@2x.png"))?;
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Vaultwrap")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "lock" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = daemon(&Request::Lock).await;
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = tauri::Emitter::emit(&w, "vault-changed", ());
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
            let _ = MouseButtonState::Up;
        })
        .build(app)?;

    // Keep the tray status line current.
    tauri::async_runtime::spawn(async move {
        loop {
            let text = match daemon(&Request::Status).await {
                Ok(r) if r.ok => match serde_json::from_value::<Status>(r.data) {
                    Ok(s) if !s.vault_exists => "Vault not set up".to_string(),
                    Ok(s) if s.unlocked => "Vault unlocked".to_string(),
                    Ok(_) => "Vault locked".to_string(),
                    Err(_) => "Daemon: unknown state".to_string(),
                },
                _ => "Daemon not running".to_string(),
            };
            let _ = status.set_text(&text);
            let _ = lock.set_enabled(text == "Vault unlocked");
            tokio::time::sleep(Duration::from_secs(4)).await;
        }
    });
    Ok(())
}

fn main() {
    let background = std::env::args().any(|a| a == "--background");
    setup::first_run();

    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            call,
            setup_info,
            install_cli,
            install_service,
            set_open_at_login,
            confirm_mac_password,
            parse_dotenv
        ])
        .setup(move |app| {
            build_tray(app.handle())?;
            if background {
                hide_main(app.handle());
            } else {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps the app in the menu bar.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide_main(window.app_handle());
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Vaultwrap");

    app.run(|app, event| {
        // Clicking the app in Finder/Dock while it's running in the menu bar.
        #[cfg(target_os = "macos")]
        if let RunEvent::Reopen { .. } = event {
            show_main(app);
        }
        let _ = (app, &event);
    });
}

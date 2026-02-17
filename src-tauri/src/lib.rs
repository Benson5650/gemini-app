mod settings;

use settings::{AppSettings, SettingsManager};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

/// 建立自訂 plugin，透過 js_init_script 在每次頁面載入時自動注入腳本，
/// 並透過 on_navigation 在 Rust 端攔截外部導航。
fn build_webview_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("gemini-webview")
        .js_init_script(include_str!("init.js").to_string())
        .on_navigation(|webview, url| {
            let host = url.host_str().unwrap_or("");
            // 允許 Tauri 內部頁面、Gemini 和 Google 登入相關域名在 webview 內導航
            if host.contains("gemini.google.com")
                || host.contains("accounts.google.com")
                || host.contains("myaccount.google.com")
                || host.contains("consent.google.com")
                || host.contains("gds.google.com")
                || host.contains("tauri.localhost")
                || host.is_empty()
                || url.scheme() == "tauri"
                || url.scheme() == "about"
                || url.scheme() == "blob"
                || url.scheme() == "data"
            {
                return true;
            }
            // 其他所有 URL 都在系統瀏覽器開啟
            let app = webview.app_handle();
            let _ = app.opener().open_url(url.as_str(), None::<&str>);
            false
        })
        .build()
}

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::RECT;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, SystemParametersInfoW, MSG, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    WM_HOTKEY,
};

const HOTKEY_PANEL_ID: i32 = 1;
const HOTKEY_NORMAL_ID: i32 = 2;

static IS_PANEL_VISIBLE: AtomicBool = AtomicBool::new(false);

fn get_work_area() -> (i32, i32, i32, i32) {
    #[cfg(target_os = "windows")]
    unsafe {
        let mut rect = RECT::default();
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        (
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        )
    }
    #[cfg(not(target_os = "windows"))]
    (0, 0, 1920, 1080)
}

fn click_latest_conversation(window: &WebviewWindow) {
    let _ = window.eval(r#"
        (function() {
            console.log("[Tauri] Script started: Auto-click latest conversation");
            
            function tryClick(attempts) {
                try {
                    // 直接查找所有對話列表中的第一個對話連結
                    // 根據結構：conversations-list -> conversations-container -> conversation-items-container -> a[data-test-id="conversation"]
                    const latestConversation = document.querySelector('conversations-list a[data-test-id="conversation"]');
                    console.log("[Tauri] Latest conversation link found?", !!latestConversation);
                    
                    if (latestConversation) {
                        latestConversation.click();
                        console.log("[Tauri] SUCCESS: Auto-clicked latest conversation:", latestConversation.href);
                        return;
                    }
                    
                    // 如果用屬性找不到，嘗試用 class
                    const itemByClass = document.querySelector('conversations-list .conversation-items-container a.conversation');
                    if (itemByClass) {
                        itemByClass.click();
                        console.log("[Tauri] SUCCESS: Auto-clicked latest conversation (by class)");
                        return;
                    }
                    
                    if (attempts > 0) {
                        console.log(`[Tauri] Element not found, retrying... (${attempts} left)`);
                        setTimeout(() => tryClick(attempts - 1), 500);
                    } else {
                        console.warn("[Tauri] FAILED: Could not find conversation item after retries");
                    }
                } catch (e) {
                    console.error("[Tauri] ERROR:", e);
                }
            }

            // 延遲一點執行，確保切換動畫完成
            setTimeout(() => tryClick(10), 200);
        })();
    "#);
}

fn toggle_normal_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("normal") {
        let settings = app.state::<SettingsManager>().get();
        if window.is_visible().unwrap_or(false) {
            // 如果已經顯示且有焦點，就隱藏
            if window.is_focused().unwrap_or(false) {
                let _ = window.hide();
            } else {
                // 如果顯示但沒前台，就帶到前台
                let _ = window.set_focus();
                // 確保 panel 關閉
                if IS_PANEL_VISIBLE.load(Ordering::SeqCst) {
                    IS_PANEL_VISIBLE.store(false, Ordering::SeqCst);
                    let app_clone = app.clone();
                    thread::spawn(move || {
                        slide_window(&app_clone, false);
                    });
                }
                if settings.normal_auto_load_conversation {
                    click_latest_conversation(&window);
                }
            }
        } else {
            // 開啟 normal window 前先關閉 panel
            if IS_PANEL_VISIBLE.load(Ordering::SeqCst) {
                IS_PANEL_VISIBLE.store(false, Ordering::SeqCst);
                let app_clone = app.clone();
                thread::spawn(move || {
                    slide_window(&app_clone, false);
                });
            }
            let _ = window.show();
            let _ = window.set_focus();
            if settings.normal_auto_load_conversation {
                click_latest_conversation(&window);
            }
        }
    }
}

fn slide_window(app: &AppHandle, show: bool) {
    let window = match app.get_webview_window("main") {
        Some(w) => w,
        None => return,
    };

    let settings = app.state::<SettingsManager>().get();
    
    // 獲取縮放比例，動態調整寬度
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let panel_width_logical = settings.panel_width;
    let panel_width_physical = (panel_width_logical as f64 * scale_factor).round() as i32;
    
    let (duration_ms, steps) = match settings.animation_speed.as_str() {
        "instant" => (1, 1), // Use 1ms to avoid division by zero
        "fast" => (100, 10),
        "normal" => (250, 20),
        "slow" => (500, 30),
        _ => (120, 15),
    };

    let (work_x, work_y, work_width, work_height) = get_work_area();
    let screen_right = work_x + work_width;

    let start_x = if show {
        screen_right
    } else {
        screen_right - panel_width_physical
    };
    let end_x = if show {
        screen_right - panel_width_physical
    } else {
        screen_right
    };

    let _ = window.set_size(PhysicalSize::new(panel_width_physical as u32, work_height as u32));

    if show {
        let _ = window.set_position(PhysicalPosition::new(start_x, work_y));
        let _ = window.show();
        let _ = window.set_focus();
    }

    if settings.animation_speed == "instant" {
        let _ = window.set_position(PhysicalPosition::new(end_x, work_y));
    } else {
        let step_delay = Duration::from_millis(duration_ms / steps as u64);
        let step_distance = (end_x - start_x) as f32 / steps as f32;

        for i in 1..=steps {
            let current_x = start_x + (step_distance * i as f32) as i32;
            let _ = window.set_position(PhysicalPosition::new(current_x, work_y));
            thread::sleep(step_delay);
        }
        let _ = window.set_position(PhysicalPosition::new(end_x, work_y));
    }

    if show {
        if settings.panel_auto_load_conversation {
            click_latest_conversation(&window);
        }
    }

    if !show {
        let _ = window.hide();
    }
}

fn toggle_panel(app: &AppHandle) {
    let is_visible = IS_PANEL_VISIBLE.load(Ordering::SeqCst);
    let new_state = !is_visible;

    // 如果要開啟 panel，先關閉 normal window
    if new_state {
        if let Some(normal) = app.get_webview_window("normal") {
            let _ = normal.hide();
        }
    }

    IS_PANEL_VISIBLE.store(new_state, Ordering::SeqCst);

    let app_clone = app.clone();
    thread::spawn(move || {
        slide_window(&app_clone, new_state);
    });
}

#[cfg(target_os = "windows")]
fn start_hotkey_listener(app: AppHandle) {
    thread::spawn(move || unsafe {
        // Register Ctrl+Alt+G for Panel
        let _ = RegisterHotKey(
            None,
            HOTKEY_PANEL_ID,
            MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
            0x47, // 'G'
        );

        // Register Ctrl+G for Normal Window
        let _ = RegisterHotKey(
            None,
            HOTKEY_NORMAL_ID,
            MOD_CONTROL | MOD_NOREPEAT,
            0x47, // 'G'
        );

        let mut msg = MSG::default();
        loop {
            if GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.message == WM_HOTKEY {
                    match msg.wParam.0 as i32 {
                        HOTKEY_PANEL_ID => toggle_panel(&app),
                        HOTKEY_NORMAL_ID => toggle_normal_window(&app),
                        _ => {}
                    }
                }
            }
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn start_hotkey_listener(_app: AppHandle) {}

// ====== Tauri Commands ======

#[tauri::command]
fn get_settings(state: State<SettingsManager>) -> AppSettings {
    state.get()
}

#[tauri::command]
fn save_settings(state: State<SettingsManager>, settings: AppSettings) -> Result<(), String> {
    state.set(settings)
}

#[tauri::command]
fn get_default_settings() -> AppSettings {
    AppSettings::default()
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())
}

fn open_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        // 創建設定視窗
        let _ = tauri::WebviewWindowBuilder::new(
            app,
            "settings",
            tauri::WebviewUrl::App("settings.html".into()),
        )
        .title("設定 - Gemini App")
        .inner_size(650.0, 700.0)
        .resizable(true)
        .center()
        .build();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(build_webview_plugin())
        .invoke_handler(tauri::generate_handler![get_settings, save_settings, get_default_settings, open_url])
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let app = window.app_handle();
                let close_to_tray = if let Some(manager) = app.try_state::<SettingsManager>() {
                    manager.get().close_to_tray
                } else {
                    true
                };

                if close_to_tray {
                    window.hide().unwrap();
                    api.prevent_close();
                }
                // If close_to_tray is false, the window will close normally,
                // and if it's the last window, the app will exit (default Tauri behavior).
            }
            _ => {}
        })
        .setup(|app| {
            let handle = app.handle();
            
            // Initialize settings manager
            let settings_manager = SettingsManager::new(handle);
            let settings = settings_manager.get();
            app.manage(settings_manager);
            
            // Apply autostart
            if settings.autostart {
                let _ = app.autolaunch().enable();
            } else {
                let _ = app.autolaunch().disable();
            }

            let quit_i = MenuItem::with_id(handle, "quit", "離開", true, None::<&str>)?;
            let show_i = MenuItem::with_id(
                handle,
                "toggle",
                "顯示/隱藏側邊欄 (Ctrl+Alt+G)",
                true,
                None::<&str>,
            )?;
            let normal_i = MenuItem::with_id(
                handle,
                "normal",
                "開啟正常視窗 (Ctrl+G)",
                true,
                None::<&str>,
            )?;
            let settings_i = MenuItem::with_id(
                handle,
                "settings",
                "設定",
                true,
                None::<&str>,
            )?;
            let menu = Menu::with_items(handle, &[&show_i, &normal_i, &settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "toggle" => toggle_panel(app),
                    "normal" => toggle_normal_window(app),
                    "settings" => open_settings_window(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        let action = app.state::<SettingsManager>().get().tray_click_action;
                        match action.as_str() {
                            "panel" => toggle_panel(app),
                            "normal" => toggle_normal_window(app),
                            "settings" => open_settings_window(app),
                            _ => toggle_panel(app),
                        }
                    }
                })
                .build(app)?;

            // Setup Panel Window
            let panel_window = app.get_webview_window("main").unwrap();
            panel_window
                .navigate(settings.gemini_url.parse().unwrap())
                .unwrap();
            let (work_x, work_y, work_width, _) = get_work_area();
            let _ = panel_window.set_position(PhysicalPosition::new(work_x + work_width, work_y));

            // Setup Normal Window
            if let Some(normal_window) = app.get_webview_window("normal") {
                normal_window
                    .navigate(settings.gemini_url.parse().unwrap())
                    .unwrap();
            }

            let app_handle = app.handle().clone();
            start_hotkey_listener(app_handle);

            // Initial state based on settings
            match settings.default_window.as_str() {
                "normal" => {
                    if let Some(normal_window) = app.get_webview_window("normal") {
                        let _ = normal_window.show();
                        let _ = normal_window.set_focus();
                        if settings.normal_auto_load_conversation {
                            click_latest_conversation(&normal_window);
                        }
                    }
                }
                "panel" => {
                    toggle_panel(handle);
                }
                _ => {} // "none" or other
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

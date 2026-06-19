use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

mod ai;
mod clipboard;
mod commands;
mod hotkeys;
mod paste;
mod secret;
mod state;
mod store;
mod tray;
mod window;

use state::AppState;
use store::Store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance 必须最先注册:第二次启动 → 唤起已运行的主窗口
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            crate::window::show_main(app);
        }))
        .plugin(tauri_plugin_clipboard::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        hotkeys::on_shortcut(app, shortcut);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let dir = app.path().app_data_dir().expect("app_data_dir");
            std::fs::create_dir_all(&dir).ok();
            let store = Store::new(&dir);
            let launch = store.get_settings().launch_at_login;
            app.manage(AppState {
                store: Mutex::new(store),
                watching: AtomicBool::new(true),
                data_dir: dir,
                last_clip: Mutex::new(String::new()),
                tray_toggle: Mutex::new(None),
            });
            clipboard::setup(app.handle());
            hotkeys::reregister(app.handle());
            tray::setup(app.handle());
            tray::apply_autostart(app.handle(), launch);
            // 主窗口:黑色 acrylic 毛玻璃(透桌面);配合 transparent + decorations:false,
            // 失败也无妨,CSS 已铺半透明深色兜底(见 main.css 的 .app 背景)
            if let Some(main) = app.get_webview_window("main") {
                let _ = window_vibrancy::apply_acrylic(&main, Some((4, 5, 7, 205)));
            }
            // 候选面板:近实色深黑材质(与 CSS 卡片同色,圆角外露出的窗口材质也不发灰)
            if let Some(panel) = app.get_webview_window("panel") {
                let _ = window_vibrancy::apply_acrylic(&panel, Some((8, 9, 11, 250)));
            }
            Ok(())
        })
        .on_window_event(|window, event| match window.label() {
            // 候选面板失焦即隐藏(复刻 Electron panel blur → hide)
            "panel" => {
                if let tauri::WindowEvent::Focused(false) = event {
                    let _ = window.hide();
                }
            }
            // 主窗口关闭:按设置最小化到托盘或退出(复刻 PRD 功能 9)
            "main" => {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let app = window.app_handle();
                    let minimize = app
                        .state::<AppState>()
                        .store
                        .lock()
                        .unwrap()
                        .get_settings()
                        .minimize_to_tray;
                    if minimize {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        app.exit(0);
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_history,
            commands::get_favorites,
            commands::get_bindings,
            commands::delete_record,
            commands::toggle_favorite,
            commands::toggle_pin,
            commands::copy_record,
            commands::copy_text,
            commands::set_binding,
            commands::unbind,
            commands::clear_history,
            commands::clear_favorites,
            commands::get_settings,
            commands::save_settings,
            commands::get_watch_state,
            commands::toggle_watch,
            commands::translate_record,
            commands::get_ai_config,
            commands::save_ai_config,
            commands::panel_search,
            commands::panel_select,
            commands::panel_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

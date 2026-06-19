//! 系统托盘 + 开机自启 + 监听状态。复刻 main.js 托盘菜单(打开/暂停恢复/退出)+ 双击 + tooltip。
use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

/// 创建托盘图标与菜单。
pub fn setup(app: &AppHandle) {
    let open = MenuItem::with_id(app, "open", "打开主窗口", true, None::<&str>);
    let toggle = MenuItem::with_id(app, "toggle", "暂停监听", true, None::<&str>);
    let sep = PredefinedMenuItem::separator(app);
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>);
    let (Ok(open), Ok(toggle), Ok(sep), Ok(quit)) = (open, toggle, sep, quit) else {
        return;
    };
    let Ok(menu) = Menu::with_items(app, &[&open, &toggle, &sep, &quit]) else {
        return;
    };

    *app.state::<AppState>().tray_toggle.lock().unwrap() = Some(toggle.clone());

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("剪贴板增强工具 · 正在监听")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => crate::window::show_main(app),
            "toggle" => {
                do_toggle_watch(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                crate::window::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    let _ = builder.build(app);
}

/// 切换监听状态(托盘菜单与前端共用):翻转 + 广播 watch-changed + 更新托盘。
pub fn do_toggle_watch(app: &AppHandle) -> bool {
    let now = {
        let state = app.state::<AppState>();
        let n = !state.watching.load(Ordering::Relaxed);
        state.watching.store(n, Ordering::Relaxed);
        n
    };
    let _ = app.emit("watch-changed", now);
    update_tooltip(app, now);
    now
}

/// 按监听状态更新托盘 tooltip 与菜单项文字。
pub fn update_tooltip(app: &AppHandle, watching: bool) {
    if let Some(item) = app.state::<AppState>().tray_toggle.lock().unwrap().as_ref() {
        let _ = item.set_text(if watching { "暂停监听" } else { "恢复监听" });
    }
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(if watching {
            "剪贴板增强工具 · 正在监听"
        } else {
            "剪贴板增强工具 · 已暂停"
        }));
    }
}

/// 应用开机自启设置(Windows = 注册表 HKCU\…\Run)。
pub fn apply_autostart(app: &AppHandle, enable: bool) {
    let m = app.autolaunch();
    let _ = if enable { m.enable() } else { m.disable() };
}

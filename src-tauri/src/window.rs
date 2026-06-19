//! 窗口管理:主窗口显示、候选面板显示/隐藏/定位(复刻 main.js showPanel)。
use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

const PANEL_CANDIDATES: usize = 10;

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn hide_panel(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("panel") {
        let _ = w.hide();
    }
}

pub fn toggle_panel(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("panel") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show_panel(app);
        }
    }
}

pub fn show_panel(app: &AppHandle) {
    let Some(w) = app.get_webview_window("panel") else {
        return;
    };
    if let Ok(cursor) = app.cursor_position() {
        position_panel(&w, cursor.x as i32, cursor.y as i32);
    }
    let data = app
        .state::<AppState>()
        .store
        .lock()
        .unwrap()
        .panel_data(PANEL_CANDIDATES);
    let _ = w.emit("panel-data", data);
    let _ = w.show();
    let _ = w.set_focus();
}

/// 鼠标位置 +10px 定位,超出所在显示器边界则回弹(复刻 Electron 多屏 + workArea 校正)。
fn position_panel(w: &WebviewWindow, cx: i32, cy: i32) {
    let size = w.outer_size().unwrap_or(PhysicalSize::new(480, 480));
    let (pw, ph) = (size.width as i32, size.height as i32);

    let monitors = w.available_monitors().unwrap_or_default();
    let bounds = monitors
        .iter()
        .find(|m| {
            let p = m.position();
            let s = m.size();
            cx >= p.x && cx < p.x + s.width as i32 && cy >= p.y && cy < p.y + s.height as i32
        })
        .or_else(|| monitors.first())
        .map(|m| {
            let p = m.position();
            let s = m.size();
            (p.x, p.y, s.width as i32, s.height as i32)
        })
        .unwrap_or((0, 0, 1920, 1080));
    let (ax, ay, aw, ah) = bounds;

    let mut x = cx + 10;
    let mut y = cy + 10;
    if x + pw > ax + aw {
        x = ax + aw - pw - 10;
    }
    if y + ph > ay + ah {
        y = ay + ah - ph - 10;
    }
    if x < ax {
        x = ax + 10;
    }
    if y < ay {
        y = ay + 10;
    }
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

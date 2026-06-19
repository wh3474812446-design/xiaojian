//! 候选面板选择 → 写剪贴板 → 隐藏面板 → 延时 → enigo 模拟 Ctrl+V。复刻 main.js panel-select。
use crate::state::AppState;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub async fn panel_select(app: AppHandle, id: String) -> Value {
    let rec = app.state::<AppState>().store.lock().unwrap().get_by_id(&id);
    let Some(rec) = rec else {
        // 记录已失效:刷新面板数据
        if let Some(w) = app.get_webview_window("panel") {
            let data = app.state::<AppState>().store.lock().unwrap().panel_data(10);
            let _ = w.emit("panel-data", data);
        }
        return json!({ "ok": false, "reason": "not_found" });
    };

    crate::clipboard::write_record_to_clipboard(&app, &rec);
    {
        let state = app.state::<AppState>();
        state.store.lock().unwrap().touch_record(&id);
        let payload = state.store.lock().unwrap().history_payload();
        let _ = app.emit("history-updated", payload);
    }

    crate::window::hide_panel(&app);
    // 隐藏后等焦点回到目标窗口,再模拟粘贴(放阻塞线程,复刻 delay(140)+SendKeys)
    let pasted = tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(Duration::from_millis(140));
        paste_ctrl_v()
    })
    .await
    .unwrap_or(false);

    json!({ "ok": true, "pasted": pasted })
}

fn paste_ctrl_v() -> bool {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let Ok(mut enigo) = Enigo::new(&Settings::default()) else {
        return false;
    };
    enigo.key(Key::Control, Direction::Press).is_ok()
        && enigo.key(Key::Unicode('v'), Direction::Click).is_ok()
        && enigo.key(Key::Control, Direction::Release).is_ok()
}

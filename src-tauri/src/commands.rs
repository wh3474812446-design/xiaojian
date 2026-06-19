//! IPC 命令(snake_case)。返回 JSON 形状与 Electron 版 IPC 完全一致。
//! 委托给其它模块的辅助函数只传 AppHandle,内部自取 AppState。

use crate::state::AppState;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

/// 改动历史/收藏/绑定后广播 history-updated(payload = historyPayload)
fn broadcast_history(app: &AppHandle, state: &AppState) {
    let payload = state.store.lock().unwrap().history_payload();
    let _ = app.emit("history-updated", payload);
}

// ---------------- 查询 ----------------
#[tauri::command]
pub fn get_history(state: State<AppState>) -> Value {
    state.store.lock().unwrap().history_payload()
}

#[tauri::command]
pub fn get_favorites(state: State<AppState>) -> Value {
    state.store.lock().unwrap().get_favorites()
}

#[tauri::command]
pub fn get_bindings(state: State<AppState>) -> Value {
    state.store.lock().unwrap().get_bindings()
}

// ---------------- 记录操作 ----------------
#[tauri::command]
pub fn delete_record(app: AppHandle, state: State<AppState>, id: String) -> Value {
    state.store.lock().unwrap().remove_record(&id);
    broadcast_history(&app, &state);
    json!({ "ok": true })
}

#[tauri::command]
pub fn toggle_favorite(app: AppHandle, state: State<AppState>, id: String) -> Value {
    let res = state.store.lock().unwrap().toggle_favorite(&id);
    broadcast_history(&app, &state);
    res
}

#[tauri::command]
pub fn toggle_pin(app: AppHandle, state: State<AppState>, id: String) -> Value {
    let res = state.store.lock().unwrap().toggle_pin(&id);
    broadcast_history(&app, &state);
    res
}

// ---------------- 复制(写剪贴板:clipboard.rs 内部已做去环) ----------------
#[tauri::command]
pub fn copy_record(app: AppHandle, state: State<AppState>, id: String) -> Value {
    let rec = state.store.lock().unwrap().touch_record(&id);
    match rec {
        Some(r) => {
            crate::clipboard::write_record_to_clipboard(&app, &r);
            broadcast_history(&app, &state);
            json!({ "ok": true })
        }
        None => json!({ "ok": false, "reason": "not_found" }),
    }
}

#[tauri::command]
pub fn copy_text(app: AppHandle, text: String) -> Value {
    // 复制纯文本(用于「复制译文」):只写剪贴板,不入历史(写回去环使监听跳过)
    crate::clipboard::write_text_to_clipboard(&app, &text);
    json!({ "ok": true })
}

// ---------------- 快捷绑定 ----------------
#[tauri::command]
pub fn set_binding(app: AppHandle, state: State<AppState>, key: String, record_id: String) -> Value {
    let res = state.store.lock().unwrap().set_binding(&key, &record_id);
    crate::hotkeys::reregister(&app);
    broadcast_history(&app, &state);
    res
}

#[tauri::command]
pub fn unbind(app: AppHandle, state: State<AppState>, key: String) -> Value {
    let res = state.store.lock().unwrap().unbind(&key);
    crate::hotkeys::reregister(&app);
    broadcast_history(&app, &state);
    res
}

// ---------------- 清空 ----------------
#[tauri::command]
pub fn clear_history(app: AppHandle, state: State<AppState>) -> Value {
    state.store.lock().unwrap().clear_history();
    broadcast_history(&app, &state);
    json!({ "ok": true })
}

#[tauri::command]
pub fn clear_favorites(app: AppHandle, state: State<AppState>) -> Value {
    state.store.lock().unwrap().clear_favorites();
    broadcast_history(&app, &state);
    json!({ "ok": true })
}

// ---------------- 设置 ----------------
#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Value {
    json!(state.store.lock().unwrap().get_settings())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<AppState>, patch: Value) -> Value {
    let res = state.store.lock().unwrap().save_settings(&patch);
    if res.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        crate::hotkeys::reregister(&app);
        if let Some(launch) = res
            .get("settings")
            .and_then(|s| s.get("launch_at_login"))
            .and_then(|v| v.as_bool())
        {
            crate::tray::apply_autostart(&app, launch);
        }
    }
    res
}

// ---------------- 监听开关 ----------------
#[tauri::command]
pub fn get_watch_state(state: State<AppState>) -> bool {
    state.watching.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn toggle_watch(app: AppHandle) -> bool {
    crate::tray::do_toggle_watch(&app)
}

// ---------------- AI 翻译(任务8接入真实) ----------------
#[tauri::command]
pub async fn translate_record(app: AppHandle, id: String) -> Value {
    crate::ai::translate_record(app, id).await
}

#[tauri::command]
pub fn get_ai_config(app: AppHandle) -> Value {
    crate::ai::get_config(&app)
}

#[tauri::command]
pub fn save_ai_config(app: AppHandle, cfg: Value) -> Value {
    crate::ai::save_config(&app, &cfg)
}

// ---------------- 候选面板 ----------------
#[tauri::command]
pub fn panel_search(state: State<AppState>, keyword: String) -> Value {
    let kw = keyword.trim().to_lowercase();
    if kw.is_empty() {
        return Value::Null;
    }
    state.store.lock().unwrap().search_view(&kw)
}

#[tauri::command]
pub async fn panel_select(app: AppHandle, id: String) -> Value {
    crate::paste::panel_select(app, id).await
}

#[tauri::command]
pub fn panel_close(app: AppHandle) {
    crate::window::hide_panel(&app);
}
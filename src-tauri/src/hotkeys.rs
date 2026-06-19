//! 全局快捷键:唤起键(可配)+ Ctrl+F1~F11 一键复制。复刻 main.js registerAllHotkeys。
use crate::state::AppState;
use crate::store::HOTKEY_KEYS;
use std::str::FromStr;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

/// 解析设置里的加速键字符串(如 "CommandOrControl+Shift+V")为 Shortcut。
fn parse_shortcut(s: &str) -> Option<Shortcut> {
    let mut mods = Modifiers::empty();
    let mut code: Option<Code> = None;
    for part in s.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "commandorcontrol" | "cmdorctrl" | "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            "super" | "meta" | "cmd" | "command" | "win" => mods |= Modifiers::SUPER,
            other => code = key_to_code(other),
        }
    }
    code.map(|c| Shortcut::new(Some(mods), c))
}

/// 单键名 → W3C Code(借 Code 的 FromStr:字母→KeyX,数字→DigitN,F1/Space 等原名)。
fn key_to_code(k: &str) -> Option<Code> {
    let up = k.to_ascii_uppercase();
    let code_str = match up.chars().next() {
        Some(c) if up.len() == 1 && c.is_ascii_alphabetic() => format!("Key{up}"),
        Some(c) if up.len() == 1 && c.is_ascii_digit() => format!("Digit{up}"),
        _ => up,
    };
    Code::from_str(&code_str).ok()
}

fn code_to_fkey(code: Code) -> Option<&'static str> {
    HOTKEY_KEYS
        .iter()
        .copied()
        .find(|k| Code::from_str(k).ok() == Some(code))
}

/// 全局键按下路由:Ctrl+F1~F11 → 一键复制;其它 → 切换候选面板。
pub fn on_shortcut(app: &AppHandle, sc: &Shortcut) {
    if sc.mods == Modifiers::CONTROL {
        if let Some(fkey) = code_to_fkey(sc.key) {
            hotkey_copy(app, fkey);
            return;
        }
    }
    crate::window::toggle_panel(app);
}

/// Ctrl+Fn:把绑定记录写入剪贴板(不自动粘贴,复刻 Electron handleHotkeyCopy)。
fn hotkey_copy(app: &AppHandle, key: &str) {
    let rec = app.state::<AppState>().store.lock().unwrap().get_bound_record(key);
    if let Some(rec) = rec {
        crate::clipboard::write_record_to_clipboard(app, &rec);
        let state = app.state::<AppState>();
        state.store.lock().unwrap().touch_record(&rec.record_id);
        let payload = state.store.lock().unwrap().history_payload();
        let _ = app.emit("history-updated", payload);
    }
}

/// 整体重注册:unregister_all 后注册唤起键 + F1..F11(任何变更都整体重注册)。
pub fn reregister(app: &AppHandle) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let hk = app
        .state::<AppState>()
        .store
        .lock()
        .unwrap()
        .get_settings()
        .global_hotkey;
    if let Some(sc) = parse_shortcut(&hk) {
        let _ = gs.register(sc);
    }
    for key in HOTKEY_KEYS {
        if let Ok(code) = Code::from_str(key) {
            let _ = gs.register(Shortcut::new(Some(Modifiers::CONTROL), code));
        }
    }
}
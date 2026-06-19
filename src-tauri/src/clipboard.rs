//! 剪贴板监听 + 读写 + 去环(全 Rust,复刻 Electron 主进程 pollClipboard 逻辑)。
//! 监听文本优先;写回后读回当前剪贴板内容作为签名,下次回调命中即跳过(防自写入回环)。
use crate::state::AppState;
use crate::store::Record;
use base64::Engine;
use std::io::Cursor;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Listener, Manager};
use tauri_plugin_clipboard::Clipboard;

const MONITOR_UPDATE: &str = "plugin:clipboard://clipboard-monitor/update";

fn b64_std() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// 启动 OS 剪贴板监听并注册变化回调。
pub fn setup(app: &AppHandle) {
    let clip = app.state::<Clipboard>();
    if let Err(e) = clip.start_monitor(app.clone()) {
        log::warn!("clipboard start_monitor failed: {e}");
    }
    let handle = app.clone();
    app.listen(MONITOR_UPDATE, move |_| on_change(&handle));
}

fn broadcast(app: &AppHandle) {
    let state = app.state::<AppState>();
    let payload = state.store.lock().unwrap().history_payload();
    let _ = app.emit("history-updated", payload);
}

fn on_change(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.watching.load(Ordering::Relaxed) {
        return;
    }
    let clip = app.state::<Clipboard>();
    // 文本优先
    if clip.has_text().unwrap_or(false) {
        if let Ok(text) = clip.read_text() {
            if !text.is_empty() {
                handle_text(app, &state, text);
                return;
            }
        }
    }
    if clip.has_image().unwrap_or(false) {
        if let Ok(img) = clip.read_image_base64() {
            if !img.is_empty() {
                handle_image(app, &state, img);
            }
        }
    }
}

fn handle_text(app: &AppHandle, state: &AppState, text: String) {
    if *state.last_clip.lock().unwrap() == text {
        return; // 自写入回环
    }
    let status = state.store.lock().unwrap().add_record(&text).status;
    if status == "saved" {
        broadcast(app);
    }
}

fn handle_image(app: &AppHandle, state: &AppState, image_b64: String) {
    if *state.last_clip.lock().unwrap() == image_b64 {
        return;
    }
    let Ok(png) = b64_std().decode(image_b64.as_bytes()) else {
        return;
    };
    let Some((thumb, w, h)) = make_thumb(&png) else {
        return;
    };
    let status = state
        .store
        .lock()
        .unwrap()
        .add_image_record(&png, thumb, w, h)
        .status;
    if status == "saved" {
        broadcast(app);
    }
}

/// 解码 PNG → 生成 220px 宽 JPEG 缩略图 dataURL,并返回原图宽高。
fn make_thumb(png: &[u8]) -> Option<(String, i64, i64)> {
    let img = image::load_from_memory(png).ok()?;
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }
    let tw = 220u32;
    let th = (((tw as f64) * (h as f64) / (w as f64)).round() as u32).max(1);
    let thumb = img.thumbnail(tw, th);
    let mut buf = Vec::new();
    thumb
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Jpeg)
        .ok()?;
    let data_url = format!("data:image/jpeg;base64,{}", b64_std().encode(&buf));
    Some((data_url, w as i64, h as i64))
}

/// 写文本到剪贴板,读回签名防回环。
pub fn write_text_to_clipboard(app: &AppHandle, text: &str) {
    let clip = app.state::<Clipboard>();
    if clip.write_text(text.to_string()).is_ok() {
        let sig = clip.read_text().unwrap_or_else(|_| text.to_string());
        *app.state::<AppState>().last_clip.lock().unwrap() = sig;
    }
}

/// 写一条记录(文本/图片)到剪贴板,读回签名防回环。
pub fn write_record_to_clipboard(app: &AppHandle, rec: &Record) {
    if rec.rec_type == "image" {
        let path = app.state::<AppState>().store.lock().unwrap().image_path(rec);
        if let Some(p) = path {
            if let Ok(bytes) = std::fs::read(&p) {
                let img_b64 = b64_std().encode(&bytes);
                let clip = app.state::<Clipboard>();
                if clip.write_image_base64(img_b64).is_ok() {
                    if let Ok(sig) = clip.read_image_base64() {
                        *app.state::<AppState>().last_clip.lock().unwrap() = sig;
                    }
                }
            }
        }
    } else {
        write_text_to_clipboard(app, &rec.content);
    }
}

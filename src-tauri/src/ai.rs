//! DeepSeek 翻译 + AI 配置(DPAPI 加密)。复刻 src/ai.js + main.js AI 部分。
use crate::secret;
use crate::state::AppState;
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";
const DEFAULT_MODEL: &str = "deepseek-v4-flash";
const ALLOWED_MODELS: [&str; 2] = ["deepseek-v4-flash", "deepseek-v4-pro"];
const MAX_TRANSLATE_CHARS: usize = 8000;
const TIMEOUT_SECS: u64 = 15;

struct AiConfig {
    enabled: bool,
    model: String,
    key: String,
}

fn config_path(app: &AppHandle) -> PathBuf {
    app.state::<AppState>().data_dir.join("ai-config.json")
}

fn load(app: &AppHandle) -> AiConfig {
    let mut c = AiConfig {
        enabled: false,
        model: DEFAULT_MODEL.to_string(),
        key: String::new(),
    };
    let Ok(raw) = std::fs::read_to_string(config_path(app)) else {
        return c;
    };
    let Ok(v) = serde_json::from_str::<Value>(&raw) else {
        return c;
    };
    c.enabled = v.get("enabled").and_then(|x| x.as_bool()).unwrap_or(false);
    if let Some(m) = v.get("model").and_then(|x| x.as_str()) {
        if ALLOWED_MODELS.contains(&m) {
            c.model = m.to_string();
        }
    }
    if let Some(enc) = v.get("key_enc").and_then(|x| x.as_str()) {
        if let Some(k) = secret::decrypt_from_base64(enc) {
            c.key = k;
        }
    } else if let Some(plain) = v.get("key_plain").and_then(|x| x.as_str()) {
        c.key = plain.to_string(); // 系统不支持加密时的降级
    }
    c
}

fn save_to_disk(app: &AppHandle, c: &AiConfig) -> bool {
    let mut out = serde_json::Map::new();
    out.insert("enabled".into(), json!(c.enabled));
    out.insert("model".into(), json!(c.model));
    if !c.key.is_empty() {
        match secret::encrypt_to_base64(&c.key) {
            Some(enc) => {
                out.insert("key_enc".into(), json!(enc));
            }
            None => {
                out.insert("key_plain".into(), json!(c.key));
            }
        }
    }
    let data = serde_json::to_vec_pretty(&Value::Object(out)).unwrap_or_default();
    let path = config_path(app);
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &data).is_ok() && std::fs::rename(&tmp, &path).is_ok()
}

fn status(c: &AiConfig) -> Value {
    json!({
        "enabled": c.enabled,
        "model": c.model,
        "hasKey": !c.key.is_empty(),
        "encryption": secret::encryption_available(),
    })
}

pub fn get_config(app: &AppHandle) -> Value {
    status(&load(app))
}

pub fn save_config(app: &AppHandle, cfg: &Value) -> Value {
    let mut c = load(app);
    if let Some(e) = cfg.get("enabled").and_then(|x| x.as_bool()) {
        c.enabled = e;
    }
    if let Some(m) = cfg.get("model").and_then(|x| x.as_str()) {
        if ALLOWED_MODELS.contains(&m) {
            c.model = m.to_string();
        }
    }
    if let Some(k) = cfg.get("apiKey").and_then(|x| x.as_str()) {
        c.key = k.to_string(); // ""=清除,非空=更新
    }
    let ok = save_to_disk(app, &c);
    let mut s = status(&c);
    s["ok"] = json!(ok);
    if !ok {
        s["reason"] = json!("write_failed");
    }
    s
}

pub async fn translate_record(app: AppHandle, id: String) -> Value {
    let c = load(&app);
    if !c.enabled {
        return json!({ "ok": false, "reason": "disabled" });
    }
    if c.key.is_empty() {
        return json!({ "ok": false, "reason": "no_key" });
    }
    let rec = app.state::<AppState>().store.lock().unwrap().get_by_id(&id);
    let Some(rec) = rec else {
        return json!({ "ok": false, "reason": "not_found" });
    };
    if rec.rec_type != "text" || rec.content.is_empty() {
        return json!({ "ok": false, "reason": "not_text" });
    }
    translate(&c.key, &c.model, &rec.content).await
}

/// 含中文 → 译英;否则 → 译中(复刻 ai.js detectTarget,CJK = U+4E00..=U+9FFF)。
fn detect_target(text: &str) -> (&'static str, &'static str) {
    let has_cjk = text.chars().any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch));
    if has_cjk {
        ("en", "English")
    } else {
        ("zh", "中文")
    }
}

async fn translate(key: &str, model: &str, text: &str) -> Value {
    let t = text.trim();
    if t.is_empty() {
        return json!({ "ok": false, "reason": "empty" });
    }
    if t.chars().count() > MAX_TRANSLATE_CHARS {
        return json!({ "ok": false, "reason": "too_long" });
    }
    let (code, label) = detect_target(t);
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": "你是专业翻译引擎。只输出翻译后的文本，不要解释、不要加引号或额外说明。" },
            { "role": "user", "content": format!("把下面的内容翻译成{label}：\n\n{t}") }
        ],
        "stream": false,
        "temperature": 1.0
    });

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(_) => return json!({ "ok": false, "reason": "network" }),
    };
    let resp = client
        .post(DEEPSEEK_URL)
        .header("Authorization", format!("Bearer {key}"))
        .json(&body)
        .send()
        .await;
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            let reason = if e.is_timeout() { "timeout" } else { "network" };
            return json!({ "ok": false, "reason": reason });
        }
    };
    let st = resp.status();
    if !st.is_success() {
        let reason = if st.as_u16() == 401 {
            "unauthorized".to_string()
        } else {
            format!("http_{}", st.as_u16())
        };
        return json!({ "ok": false, "reason": reason });
    }
    let data: Value = match resp.json().await {
        Ok(d) => d,
        Err(_) => return json!({ "ok": false, "reason": "network" }),
    };
    let out = data
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if out.is_empty() {
        return json!({ "ok": false, "reason": "empty_result" });
    }
    json!({ "ok": true, "text": out, "target": code })
}

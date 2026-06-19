//! 本地数据存储 —— store.js 的 Rust 等价实现(schema / 业务规则 1:1)。
//!
//! 历史/设置/绑定以 JSON 保存,图片原图以 PNG 文件存于 images 子目录。
//! 关键规则(与 Electron 版一致):
//! - 收藏记录受保护:不被「清空普通历史」删除、不计入容量上限、不被容量裁剪。
//! - 仅收藏记录可置顶;取消收藏会同时取消置顶。
//! - 绑定指向 record_id;记录被删除/裁剪后绑定自动失效并清理。
//! - 序列化输出的 JSON 形状与旧版完全一致 → 数据文件互通。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_CONTENT_LENGTH: usize = 50_000; // content 最大字符数(UTF-16 计)
const PREVIEW_LENGTH: usize = 80;
const ALLOWED_MAX_RECORDS: [i64; 4] = [50, 100, 200, 500];
pub const HOTKEY_KEYS: [&str; 11] = [
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11",
];

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ---------------- 记录结构 ----------------
// 文本记录与图片记录共用一个结构:图片专属字段用 Option + skip_serializing_if,
// 使文本记录序列化时不输出这些键 → 与旧版 JSON 形状逐字一致。
#[derive(Clone, Serialize, Deserialize)]
pub struct Record {
    pub record_id: String,
    #[serde(rename = "type")]
    pub rec_type: String, // "text" | "image"
    #[serde(default)]
    pub content: String,
    pub preview: String,
    pub copied_at: i64,
    #[serde(default)]
    pub char_count: i64,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub favorited_at: Option<i64>,
    #[serde(default)]
    pub pinned_at: Option<i64>,
    // ----- 图片专属(文本记录为 None,不序列化) -----
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<i64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub global_hotkey: String,
    pub max_records: i64,
    pub launch_at_login: bool,
    pub minimize_to_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            global_hotkey: "CommandOrControl+Shift+V".into(),
            max_records: 100,
            launch_at_login: false,
            minimize_to_tray: true,
        }
    }
}

/// add_record / add_image_record 的结果
pub struct AddOutcome {
    pub status: &'static str, // "saved" | "ignored" | "too_long" | "error"
}

// ---------------- 视图投影(发给前端,图片不含原图大数据/不含 hash·file 内部字段) ----------------
fn to_view(r: &Record) -> Value {
    let mut v = json!({
        "record_id": r.record_id,
        "type": r.rec_type,
        "preview": r.preview,
        "copied_at": r.copied_at,
        "is_favorite": r.is_favorite,
        "is_pinned": r.is_pinned,
        "favorited_at": r.favorited_at,
        "pinned_at": r.pinned_at,
    });
    let o = v.as_object_mut().unwrap();
    if r.rec_type == "image" {
        o.insert("thumb".into(), json!(r.thumb));
        o.insert("width".into(), json!(r.width));
        o.insert("height".into(), json!(r.height));
        o.insert("byte_size".into(), json!(r.byte_size));
        o.insert("content".into(), json!(""));
        o.insert("char_count".into(), json!(0));
    } else {
        o.insert("content".into(), json!(r.content));
        o.insert("char_count".into(), json!(r.char_count));
    }
    v
}

fn make_preview(content: &str) -> String {
    // 等价 content.replace(/\s+/g,' ').trim() 再截前 80 字符
    let one_line = content.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(PREVIEW_LENGTH).collect()
}

fn utf16_len(s: &str) -> i64 {
    s.encode_utf16().count() as i64 // 与 JS string.length 一致
}

fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut tmp_os: OsString = path.as_os_str().to_owned();
    tmp_os.push(".tmp");
    let tmp = PathBuf::from(tmp_os);
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

// ---------------- Store ----------------
pub struct Store {
    history_file: PathBuf,
    settings_file: PathBuf,
    bindings_file: PathBuf,
    images_dir: PathBuf,
    history: Vec<Record>,
    settings: Settings,
    bindings: BTreeMap<String, String>, // 仅保存已绑定项
}

impl Store {
    pub fn new(user_data: &Path) -> Self {
        let mut s = Store {
            history_file: user_data.join("clipboard-history.json"),
            settings_file: user_data.join("settings.json"),
            bindings_file: user_data.join("hotkey-bindings.json"),
            images_dir: user_data.join("images"),
            history: Vec::new(),
            settings: Settings::default(),
            bindings: BTreeMap::new(),
        };
        let _ = fs::create_dir_all(&s.images_dir);
        s.load_settings();
        s.load_history();
        s.load_bindings();
        s
    }

    // ---------- 加载 ----------
    fn load_settings(&mut self) {
        match fs::read_to_string(&self.settings_file) {
            Ok(raw) => {
                if let Ok(mut st) = serde_json::from_str::<Settings>(&raw) {
                    if !ALLOWED_MAX_RECORDS.contains(&st.max_records) {
                        st.max_records = Settings::default().max_records;
                    }
                    // minimize_to_tray 默认 true:仅显式 false 才关闭
                    self.settings = st;
                } else {
                    self.settings = Settings::default();
                }
            }
            Err(_) => {
                let _ = self.write_settings();
            }
        }
    }

    fn load_history(&mut self) {
        let Ok(raw) = fs::read_to_string(&self.history_file) else {
            return;
        };
        let Ok(arr) = serde_json::from_str::<Vec<Record>>(&raw) else {
            return;
        };
        self.history = arr
            .into_iter()
            .map(normalize_record)
            .filter(|r| {
                if r.rec_type == "text" {
                    !r.content.is_empty()
                } else if r.rec_type == "image" {
                    match &r.image_file {
                        Some(f) => self.images_dir.join(f).exists(),
                        None => false,
                    }
                } else {
                    false
                }
            })
            .collect();
    }

    fn load_bindings(&mut self) {
        let Ok(raw) = fs::read_to_string(&self.bindings_file) else {
            return;
        };
        let Ok(map) = serde_json::from_str::<BTreeMap<String, Value>>(&raw) else {
            return;
        };
        for key in HOTKEY_KEYS {
            if let Some(Value::String(rid)) = map.get(key) {
                self.bindings.insert(key.to_string(), rid.clone());
            }
        }
    }

    // ---------- 写入(原子) ----------
    fn write_history(&self) -> std::io::Result<()> {
        let data = serde_json::to_vec(&self.history).unwrap_or_default();
        atomic_write(&self.history_file, &data)
    }
    fn write_settings(&self) -> std::io::Result<()> {
        let data = serde_json::to_vec_pretty(&self.settings).unwrap_or_default();
        atomic_write(&self.settings_file, &data)
    }
    fn write_bindings(&self) -> std::io::Result<()> {
        let data = serde_json::to_vec_pretty(&self.bindings).unwrap_or_default();
        atomic_write(&self.bindings_file, &data)
    }

    // ---------- 文本记录 ----------
    pub fn add_record(&mut self, content: &str) -> AddOutcome {
        if content.is_empty() {
            return AddOutcome { status: "ignored" };
        }
        if utf16_len(content) as usize > MAX_CONTENT_LENGTH {
            return AddOutcome { status: "too_long" };
        }
        if let Some(idx) = self
            .history
            .iter()
            .position(|r| r.rec_type == "text" && r.content == content)
        {
            self.touch_at(idx);
            return AddOutcome { status: "saved" };
        }
        let record = Record {
            record_id: uuid::Uuid::new_v4().to_string(),
            rec_type: "text".into(),
            content: content.to_string(),
            preview: make_preview(content),
            copied_at: now_ms(),
            char_count: utf16_len(content),
            is_favorite: false,
            is_pinned: false,
            favorited_at: None,
            pinned_at: None,
            image_hash: None,
            image_file: None,
            thumb: None,
            width: None,
            height: None,
            byte_size: None,
        };
        self.history.insert(0, record);
        self.trim();
        let _ = self.write_history();
        AddOutcome { status: "saved" }
    }

    // ---------- 图片记录 ----------
    pub fn add_image_record(
        &mut self,
        png: &[u8],
        thumb: String,
        width: i64,
        height: i64,
    ) -> AddOutcome {
        if png.is_empty() {
            return AddOutcome { status: "ignored" };
        }
        let hash = sha1_smol::Sha1::from(png).digest().to_string();
        let file_name = format!("{hash}.png");
        let file_path = self.images_dir.join(&file_name);

        if let Some(idx) = self
            .history
            .iter()
            .position(|r| r.rec_type == "image" && r.image_hash.as_deref() == Some(hash.as_str()))
        {
            self.touch_at(idx);
            return AddOutcome { status: "saved" };
        }

        let _ = fs::create_dir_all(&self.images_dir);
        if !file_path.exists() {
            if fs::write(&file_path, png).is_err() {
                return AddOutcome { status: "error" };
            }
        }

        let record = Record {
            record_id: uuid::Uuid::new_v4().to_string(),
            rec_type: "image".into(),
            content: String::new(),
            preview: format!("图片 {width}×{height}"),
            copied_at: now_ms(),
            char_count: 0,
            is_favorite: false,
            is_pinned: false,
            favorited_at: None,
            pinned_at: None,
            image_hash: Some(hash),
            image_file: Some(file_name),
            thumb: Some(thumb),
            width: Some(width),
            height: Some(height),
            byte_size: Some(png.len() as i64),
        };
        self.history.insert(0, record);
        self.trim();
        let _ = self.write_history();
        AddOutcome { status: "saved" }
    }

    /// 把指定下标记录刷新复制时间并置顶,保留全部状态
    fn touch_at(&mut self, idx: usize) -> Record {
        let mut rec = self.history.remove(idx);
        rec.copied_at = now_ms();
        self.history.insert(0, rec.clone());
        let _ = self.write_history();
        rec
    }

    pub fn touch_record(&mut self, record_id: &str) -> Option<Record> {
        let idx = self.history.iter().position(|r| r.record_id == record_id)?;
        Some(self.touch_at(idx))
    }

    pub fn get_by_id(&self, record_id: &str) -> Option<Record> {
        self.history.iter().find(|r| r.record_id == record_id).cloned()
    }

    pub fn image_path(&self, rec: &Record) -> Option<PathBuf> {
        if rec.rec_type == "image" {
            rec.image_file.as_ref().map(|f| self.images_dir.join(f))
        } else {
            None
        }
    }

    /// 容量裁剪:仅裁非收藏记录,从最旧开始,直到非收藏数 <= max_records
    fn trim(&mut self) {
        let max = self.settings.max_records;
        let mut non_fav = self.history.iter().filter(|r| !r.is_favorite).count() as i64;
        if non_fav <= max {
            return;
        }
        let mut bindings_changed = false;
        let mut i = self.history.len();
        while i > 0 && non_fav > max {
            i -= 1;
            if self.history[i].is_favorite {
                continue;
            }
            let removed = self.history.remove(i);
            if self.clean_bindings_for_record(&removed.record_id) {
                bindings_changed = true;
            }
            self.maybe_delete_image_file(&removed);
            non_fav -= 1;
        }
        if bindings_changed {
            let _ = self.write_bindings();
        }
    }

    fn maybe_delete_image_file(&self, rec: &Record) {
        if rec.rec_type != "image" {
            return;
        }
        let Some(file) = &rec.image_file else { return };
        let still_used = self
            .history
            .iter()
            .any(|r| r.rec_type == "image" && r.image_file.as_deref() == Some(file.as_str()));
        if !still_used {
            let _ = fs::remove_file(self.images_dir.join(file));
        }
    }

    // ---------- 收藏 / 置顶 ----------
    pub fn toggle_favorite(&mut self, record_id: &str) -> Value {
        let Some(rec) = self.history.iter_mut().find(|r| r.record_id == record_id) else {
            return json!({"ok": false, "reason": "not_found"});
        };
        rec.is_favorite = !rec.is_favorite;
        if rec.is_favorite {
            rec.favorited_at = Some(now_ms());
        } else {
            rec.favorited_at = None;
            rec.is_pinned = false;
            rec.pinned_at = None;
        }
        let fav = rec.is_favorite;
        if self.write_history().is_err() {
            return json!({"ok": false, "reason": "write_failed"});
        }
        json!({"ok": true, "is_favorite": fav})
    }

    pub fn toggle_pin(&mut self, record_id: &str) -> Value {
        let Some(rec) = self.history.iter_mut().find(|r| r.record_id == record_id) else {
            return json!({"ok": false, "reason": "not_found"});
        };
        if !rec.is_favorite {
            return json!({"ok": false, "reason": "not_favorite"});
        }
        rec.is_pinned = !rec.is_pinned;
        rec.pinned_at = if rec.is_pinned { Some(now_ms()) } else { None };
        let pinned = rec.is_pinned;
        if self.write_history().is_err() {
            return json!({"ok": false, "reason": "write_failed"});
        }
        json!({"ok": true, "is_pinned": pinned})
    }

    // ---------- 视图查询 ----------
    pub fn history_payload(&self) -> Value {
        json!({
            "history": self.history.iter().map(to_view).collect::<Vec<_>>(),
            "count": self.history.len(),
            "historyCount": self.history_count(),
            "favoriteCount": self.favorite_count(),
            "max": self.settings.max_records,
        })
    }

    pub fn get_favorites(&self) -> Value {
        let mut pinned: Vec<&Record> =
            self.history.iter().filter(|r| r.is_favorite && r.is_pinned).collect();
        pinned.sort_by(|a, b| b.pinned_at.unwrap_or(0).cmp(&a.pinned_at.unwrap_or(0)));
        let mut normal: Vec<&Record> =
            self.history.iter().filter(|r| r.is_favorite && !r.is_pinned).collect();
        normal.sort_by(|a, b| b.favorited_at.unwrap_or(0).cmp(&a.favorited_at.unwrap_or(0)));
        json!({
            "pinned": pinned.iter().map(|r| to_view(r)).collect::<Vec<_>>(),
            "normal": normal.iter().map(|r| to_view(r)).collect::<Vec<_>>(),
        })
    }

    pub fn get_recent(&self, n: usize) -> Vec<Value> {
        self.history.iter().take(n).map(to_view).collect()
    }

    /// 候选面板数据:{recent, favorites}
    pub fn panel_data(&self, recent_n: usize) -> Value {
        let fav = self.get_favorites();
        // favorites 给面板用置顶+普通合并(置顶在前)
        let mut favorites: Vec<Value> = Vec::new();
        if let Some(p) = fav.get("pinned").and_then(|v| v.as_array()) {
            favorites.extend(p.iter().cloned());
        }
        if let Some(nv) = fav.get("normal").and_then(|v| v.as_array()) {
            favorites.extend(nv.iter().cloned());
        }
        favorites.truncate(recent_n); // 收藏在面板的展示上限(与 recent 同)
        json!({ "recent": self.get_recent(recent_n), "favorites": favorites })
    }

    /// 候选面板搜索:复刻 Electron panel-search —— 仅文本、按 content 匹配、上限 20。
    pub fn search_view(&self, kw_lower: &str) -> Value {
        let list: Vec<Value> = self
            .history
            .iter()
            .filter(|r| r.rec_type == "text" && r.content.to_lowercase().contains(kw_lower))
            .take(20)
            .map(to_view)
            .collect();
        json!(list)
    }

    pub fn remove_record(&mut self, record_id: &str) -> bool {
        let Some(idx) = self.history.iter().position(|r| r.record_id == record_id) else {
            return false;
        };
        let removed = self.history.remove(idx);
        self.maybe_delete_image_file(&removed);
        let _ = self.write_history();
        if self.clean_bindings_for_record(&removed.record_id) {
            let _ = self.write_bindings();
        }
        true
    }

    pub fn clear_history(&mut self) {
        let removed: Vec<Record> =
            self.history.iter().filter(|r| !r.is_favorite).cloned().collect();
        self.history.retain(|r| r.is_favorite);
        let _ = self.write_history();
        let mut changed = false;
        for r in &removed {
            if self.clean_bindings_for_record(&r.record_id) {
                changed = true;
            }
        }
        if changed {
            let _ = self.write_bindings();
        }
        self.gc_images();
    }

    pub fn clear_favorites(&mut self) {
        let removed: Vec<Record> =
            self.history.iter().filter(|r| r.is_favorite).cloned().collect();
        self.history.retain(|r| !r.is_favorite);
        let _ = self.write_history();
        let mut changed = false;
        for r in &removed {
            if self.clean_bindings_for_record(&r.record_id) {
                changed = true;
            }
        }
        if changed {
            let _ = self.write_bindings();
        }
        self.gc_images();
    }

    fn gc_images(&self) {
        let used: std::collections::HashSet<&str> = self
            .history
            .iter()
            .filter_map(|r| r.image_file.as_deref())
            .collect();
        if let Ok(entries) = fs::read_dir(&self.images_dir) {
            for e in entries.flatten() {
                let name = e.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(".png") && !used.contains(name.as_ref()) {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
    }

    pub fn history_count(&self) -> i64 {
        self.history.iter().filter(|r| !r.is_favorite).count() as i64
    }
    pub fn favorite_count(&self) -> i64 {
        self.history.iter().filter(|r| r.is_favorite).count() as i64
    }

    // ---------- 绑定 ----------
    pub fn get_bindings(&mut self) -> Value {
        let mut out = serde_json::Map::new();
        let mut changed = false;
        for key in HOTKEY_KEYS {
            match self.bindings.get(key).cloned() {
                None => {
                    out.insert(key.into(), json!({"record_id": null, "bound": false}));
                }
                Some(rid) => match self.history.iter().find(|r| r.record_id == rid) {
                    None => {
                        self.bindings.remove(key);
                        changed = true;
                        out.insert(key.into(), json!({"record_id": null, "bound": false}));
                    }
                    Some(rec) => {
                        let thumb = if rec.rec_type == "image" { rec.thumb.clone() } else { None };
                        out.insert(
                            key.into(),
                            json!({
                                "record_id": rid,
                                "bound": true,
                                "preview": rec.preview,
                                "type": rec.rec_type,
                                "thumb": thumb,
                            }),
                        );
                    }
                },
            }
        }
        if changed {
            let _ = self.write_bindings();
        }
        Value::Object(out)
    }

    pub fn set_binding(&mut self, key: &str, record_id: &str) -> Value {
        if !HOTKEY_KEYS.contains(&key) {
            return json!({"ok": false, "reason": "invalid_hotkey"});
        }
        if !self.history.iter().any(|r| r.record_id == record_id) {
            return json!({"ok": false, "reason": "record_not_found"});
        }
        self.bindings.insert(key.to_string(), record_id.to_string());
        if self.write_bindings().is_err() {
            return json!({"ok": false, "reason": "write_failed"});
        }
        json!({"ok": true})
    }

    pub fn unbind(&mut self, key: &str) -> Value {
        if !HOTKEY_KEYS.contains(&key) {
            return json!({"ok": false, "reason": "invalid_hotkey"});
        }
        if self.bindings.remove(key).is_some() && self.write_bindings().is_err() {
            return json!({"ok": false, "reason": "write_failed"});
        }
        json!({"ok": true})
    }

    pub fn get_bound_record(&mut self, key: &str) -> Option<Record> {
        let rid = self.bindings.get(key).cloned()?;
        match self.history.iter().find(|r| r.record_id == rid) {
            Some(rec) => Some(rec.clone()),
            None => {
                self.bindings.remove(key);
                let _ = self.write_bindings();
                None
            }
        }
    }

    fn clean_bindings_for_record(&mut self, record_id: &str) -> bool {
        let before = self.bindings.len();
        self.bindings.retain(|_, v| v != record_id);
        before != self.bindings.len()
    }

    // ---------- 设置 ----------
    pub fn get_settings(&self) -> Settings {
        self.settings.clone()
    }

    pub fn save_settings(&mut self, patch: &Value) -> Value {
        let mut next = self.settings.clone();
        if let Some(v) = patch.get("global_hotkey").and_then(|v| v.as_str()) {
            next.global_hotkey = v.to_string();
        }
        if let Some(v) = patch.get("max_records").and_then(|v| v.as_i64()) {
            next.max_records = v;
        }
        if let Some(v) = patch.get("launch_at_login").and_then(|v| v.as_bool()) {
            next.launch_at_login = v;
        }
        if let Some(v) = patch.get("minimize_to_tray").and_then(|v| v.as_bool()) {
            next.minimize_to_tray = v;
        }
        if !ALLOWED_MAX_RECORDS.contains(&next.max_records) {
            return json!({"ok": false, "reason": "invalid_max_records", "settings": self.get_settings()});
        }
        if next.global_hotkey.trim().is_empty() {
            return json!({"ok": false, "reason": "empty_hotkey", "settings": self.get_settings()});
        }
        self.settings = next;
        if self.write_settings().is_err() {
            return json!({"ok": false, "reason": "write_failed", "settings": self.get_settings()});
        }
        self.trim();
        let _ = self.write_history();
        json!({"ok": true, "settings": self.get_settings()})
    }
}

// 兼容旧数据:补默认字段(无 type 视为文本)
fn normalize_record(mut r: Record) -> Record {
    if r.rec_type.is_empty() {
        r.rec_type = "text".into();
    }
    r
}

//! 全局应用状态(Tauri managed state)。
use crate::store::Store;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::menu::MenuItem;
use tauri::Wry;

pub struct AppState {
    pub store: Mutex<Store>,
    pub watching: AtomicBool,     // 是否正在监听剪贴板
    pub data_dir: PathBuf,        // 数据目录(ai-config.json 等定位用)
    pub last_clip: Mutex<String>, // 程序最后写入剪贴板的内容签名(防自写入回环)
    pub tray_toggle: Mutex<Option<MenuItem<Wry>>>, // 托盘「暂停/恢复监听」菜单项(动态改文字)
}

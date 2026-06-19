//! DPAPI 加解密 —— 替代 Electron safeStorage。
//! 用 Scope::User + 无 entropy,与 safeStorage(Windows CryptProtectData)一致,可解旧 key_enc。
use base64::Engine;
use windows_dpapi::{decrypt_data, encrypt_data, Scope};

fn b64() -> base64::engine::general_purpose::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// Windows 上 DPAPI 始终可用。
pub fn encryption_available() -> bool {
    true
}

pub fn encrypt_to_base64(plain: &str) -> Option<String> {
    encrypt_data(plain.as_bytes(), Scope::User, None)
        .ok()
        .map(|v| b64().encode(v))
}

pub fn decrypt_from_base64(b64_str: &str) -> Option<String> {
    let bytes = b64().decode(b64_str).ok()?;
    let plain = decrypt_data(&bytes, Scope::User, None).ok()?;
    String::from_utf8(plain).ok()
}

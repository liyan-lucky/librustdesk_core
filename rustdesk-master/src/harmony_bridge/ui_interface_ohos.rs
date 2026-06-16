use hbb_common::config::{Config, LocalConfig};
use serde_derive::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct LoginDeviceInfo {
    pub os: String,
    pub r#type: String,
    pub name: String,
}

pub fn get_option<T: AsRef<str>>(key: T) -> String {
    Config::get_option(key.as_ref())
}

pub fn set_option(key: String, value: String) {
    Config::set_option(key, value);
}

pub fn get_local_option(key: String) -> String {
    LocalConfig::get_option(&key)
}

pub fn set_local_option(key: String, value: String) {
    LocalConfig::set_option(key, value);
}

pub fn get_builtin_option(key: &str) -> String {
    crate::common::get_builtin_option(key)
}

pub fn use_texture_render() -> bool {
    false
}

pub fn resolve_avatar_url(avatar: String) -> String {
    avatar
}

pub fn video_save_directory(_root: bool) -> String {
    String::new()
}

pub fn handle_relay_id(id: &str) -> &str {
    id
}

pub fn get_api_server() -> String {
    Config::get_option("api-server")
}

pub fn is_installed() -> bool {
    false
}

pub fn get_login_device_info() -> LoginDeviceInfo {
    LoginDeviceInfo {
        os: "ohos".to_owned(),
        r#type: "client".to_owned(),
        name: crate::common::hostname(),
    }
}

pub fn get_login_device_info_json() -> String {
    serde_json::to_string(&get_login_device_info()).unwrap_or_else(|_| "{}".to_owned())
}

pub fn max_encrypt_len() -> usize {
    hbb_common::config::ENCRYPT_MAX_LEN
}

pub fn get_options() -> String {
    "{}".to_owned()
}

pub fn set_options(options: HashMap<String, String>) {
    for (key, value) in options {
        Config::set_option(key, value);
    }
}

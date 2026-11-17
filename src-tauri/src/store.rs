use crate::types::{Workspace, AppSettings};
use std::{fs, path::PathBuf};
use tauri::Manager;

pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("portable.dat").exists() { return dir.join("data"); }
        }
    }
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn workspace_dir(app: &tauri::AppHandle) -> PathBuf {
    let dir = data_dir(app).join("workspace");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn settings_path(app: &tauri::AppHandle) -> PathBuf { data_dir(app).join("settings.json") }

pub fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    fs::read_to_string(settings_path(app)).ok()
        .and_then(|s| serde_json::from_str::<AppSettings>(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(app: &tauri::AppHandle, s: &AppSettings) -> Result<(), String> {
    fs::write(settings_path(app), serde_json::to_string_pretty(s).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// 集合以纯文本 JSON 文件保存：一个集合一个目录，每个请求一个 .json 文件。
/// 这样可以直接放进 Git 仓库做版本控制，也便于 diff。
pub fn load_workspace(app: &tauri::AppHandle) -> Workspace {
    let root = workspace_dir(app);
    let mut ws = Workspace::default();
    if let Ok(entries) = fs::read_dir(&root) {
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() { continue; }
            let name = match p.file_name().and_then(|n| n.to_str()) { Some(n) => n.to_string(), None => continue };
            if name.starts_with('.') { continue; }
            let mut collection = crate::types::Collection {
                id: name.clone(), name, requests: Vec::new(),
            };
            if let Ok(files) = fs::read_dir(&p) {
                for f in files.flatten() {
                    let fp = f.path();
                    if fp.extension().and_then(|x| x.to_str()) != Some("json") { continue; }
                    if let Ok(txt) = fs::read_to_string(&fp) {
                        if let Ok(mut req) = serde_json::from_str::<crate::types::ApiRequest>(&txt) {
                            if req.id.is_empty() {
                                req.id = fp.file_stem().and_then(|s| s.to_str()).unwrap_or("req").to_string();
                            }
                            collection.requests.push(req);
                        }
                    }
                }
            }
            collection.requests.sort_by(|a, b| a.name.cmp(&b.name));
            ws.collections.push(collection);
        }
    }
    ws.collections.sort_by(|a, b| a.name.cmp(&b.name));
    // 环境变量：workspace/.env.json（不进集合目录）
    let env_path = root.join(".env.json");
    if let Ok(txt) = fs::read_to_string(&env_path) {
        if let Ok(envs) = serde_json::from_str(&txt) { ws.environments = envs; }
    }
    ws
}

pub fn save_request(app: &tauri::AppHandle, collection_id: &str, req: &crate::types::ApiRequest) -> Result<(), String> {
    let dir = workspace_dir(app).join(collection_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe = req.id.replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_");
    let path = dir.join(format!("{}.json", safe));
    fs::write(path, serde_json::to_string_pretty(req).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub fn delete_request(app: &tauri::AppHandle, collection_id: &str, id: &str) -> Result<(), String> {
    let path = workspace_dir(app).join(collection_id).join(format!("{}.json", id.replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_")));
    if path.exists() { fs::remove_file(path).map_err(|e| e.to_string())?; }
    Ok(())
}

pub fn create_collection(app: &tauri::AppHandle, name: &str) -> Result<(), String> {
    let dir = workspace_dir(app).join(name);
    fs::create_dir_all(dir).map_err(|e| e.to_string())
}

pub fn save_environments(app: &tauri::AppHandle, envs: &std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>) -> Result<(), String> {
    let path = workspace_dir(app).join(".env.json");
    fs::write(path, serde_json::to_string_pretty(envs).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub fn open_workspace_dir(app: &tauri::AppHandle) -> Result<(), String> {
    let dir = workspace_dir(app);
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer.exe").arg(&dir).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

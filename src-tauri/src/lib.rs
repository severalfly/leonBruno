mod bru;
mod curl;
mod yamlreq;
mod http;
mod store;
mod types;

use tauri::Manager;
use types::{ApiRequest, ApiResponse, AppSettings, Workspace};

#[tauri::command]
fn list_workspace(app: tauri::AppHandle) -> Workspace {
    store::load_workspace(&app)
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> AppSettings {
    store::load_settings(&app)
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    store::save_settings(&app, &settings)
}

#[tauri::command]
fn probe_bruno_repo(path: String) -> Result<(String, usize), String> {
    store::probe_bruno_repo(&path)
}
#[tauri::command]
fn list_extra_roots(app: tauri::AppHandle) -> Vec<String> { store::load_extra_roots(&app) }
#[tauri::command]
fn add_extra_root(app: tauri::AppHandle, path: String) -> Result<Vec<String>, String> {
    let mut roots = store::load_extra_roots(&app);
    if !roots.contains(&path) { roots.push(path); }
    store::save_extra_roots(&app, &roots)?;
    Ok(roots)
}
#[tauri::command]
fn remove_extra_root(app: tauri::AppHandle, path: String) -> Result<Vec<String>, String> {
    let mut roots = store::load_extra_roots(&app);
    roots.retain(|r| r != &path);
    store::save_extra_roots(&app, &roots)?;
    Ok(roots)
}
#[tauri::command]
fn parse_curl_command(input: String) -> Result<ApiRequest, String> {
    curl::parse_curl(&input)
}

#[tauri::command]
fn save_request(app: tauri::AppHandle, collection: String, request: ApiRequest) -> Result<(), String> {
    store::save_request(&app, &collection, &request)
}

#[tauri::command]
fn delete_request(app: tauri::AppHandle, collection: String, id: String) -> Result<(), String> {
    store::delete_request(&app, &collection, &id)
}

#[tauri::command]
fn create_collection(app: tauri::AppHandle, name: String) -> Result<(), String> {
    store::create_collection(&app, &name)
}

#[tauri::command]
fn save_environments(app: tauri::AppHandle, envs: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>) -> Result<(), String> {
    store::save_environments(&app, &envs)
}

#[tauri::command]
fn open_workspace_dir(app: tauri::AppHandle) -> Result<(), String> {
    store::open_workspace_dir(&app)
}

#[tauri::command]
async fn send_request(request: ApiRequest, env: std::collections::BTreeMap<String, String>, timeout_secs: u64) -> Result<ApiResponse, String> {
    http::execute(request, env, timeout_secs).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let _ = store::workspace_dir(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_workspace, get_settings, save_settings, parse_curl_command,
            save_request, delete_request, create_collection, save_environments,
            open_workspace_dir, send_request, probe_bruno_repo, list_extra_roots,
            add_extra_root, remove_extra_root
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

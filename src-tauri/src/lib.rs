use serde::Serialize;
use std::sync::Mutex;
use tauri::Manager;

mod db;
pub mod model;
mod projects;
mod settings;

/// App identity returned to the frontend over the Tauri bridge.
#[derive(Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

/// Frontend health round-trip: returns the app name + version so the shell can
/// prove the React -> Rust bridge is live. Takes no arguments and cannot fail.
#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "Review Helper".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Open + migrate the SQLite database, then hand the connection to
            // Tauri's managed state so every command can reach it.
            let conn = db::connect_app_db(app.handle())?;
            app.manage(db::Db(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            model::commands::model_run,
            settings::get_model_config,
            settings::set_model_config,
            projects::create_project,
            projects::list_projects,
            projects::get_project,
            projects::rename_project,
            projects::delete_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reports_name_and_version() {
        let info = app_info();
        assert_eq!(info.name, "Review Helper");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert!(!info.version.is_empty());
    }
}

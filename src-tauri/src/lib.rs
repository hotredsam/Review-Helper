use serde::Serialize;
use std::sync::Mutex;
use tauri::Manager;

mod assess;
mod audit;
mod cards;
mod chat;
pub mod context;
mod decisions;
mod features;
mod db;
mod github;
mod grill;
pub mod model;
mod plan;
mod projects;
mod stack;
mod suggestions;
mod sync;
mod util;
#[cfg(test)]
mod seed_real;
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
            let _ = cards::seed(&conn); // best-effort seed of the curated cards
            app.manage(db::Db(Mutex::new(conn)));
            app.manage(cards::commands::CardGate::default());
            app.manage(grill::commands::GrillGate::default());
            app.manage(plan::commands::PlanGate::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            model::commands::model_run,
            model::commands::model_status,
            settings::get_model_config,
            settings::set_model_config,
            github::commands::github_status,
            github::commands::github_connect_gh,
            github::commands::github_sign_out,
            github::commands::github_list_repos,
            github::commands::github_device_start,
            github::commands::github_device_poll,
            github::commands::project_import_repo,
            github::commands::project_link_url,
            github::commands::project_create_repo,
            github::commands::project_clone,
            plan::commands::analyze_project,
            plan::commands::kickoff_project,
            plan::commands::update_plan,
            plan::commands::rebuild_plan,
            plan::commands::get_plan,
            audit::commands::audit_list,
            sync::commands::sync_package,
            sync::commands::sync_push_planning,
            sync::commands::sync_main_preview,
            sync::commands::sync_main_apply,
            assess::commands::assess_project,
            assess::commands::get_assessment,
            cards::commands::cards_list,
            cards::commands::card_get,
            cards::commands::card_explain,
            cards::commands::card_capture,
            grill::commands::grill_generate,
            grill::commands::grill_list,
            grill::commands::grill_answer,
            grill::commands::grill_chat_resolve,
            grill::commands::grill_set_status,
            grill::commands::grill_delete,
            chat::commands::chat_send,
            suggestions::commands::suggestions_list,
            suggestions::commands::suggestion_approve,
            suggestions::commands::suggestion_dismiss,
            suggestions::commands::suggestions_approve_all,
            decisions::commands::decisions_list,
            decisions::commands::decision_supersede,
            features::commands::features_list,
            features::commands::feature_add,
            features::commands::feature_set_status,
            features::commands::features_pending_count,
            features::commands::transcribe_audio_stub,
            stack::commands::stack_catalog,
            stack::commands::stack_premade,
            stack::commands::stack_list,
            stack::commands::stack_set,
            stack::commands::stack_apply_premade,
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

mod catalogs;
mod commands;
mod domain;
mod repositories;
mod services;
mod system;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let pool = tauri::async_runtime::block_on(repositories::database::open(
                &data_dir.join("meelting.db"),
            ))?;
            app.manage(pool);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::keychain::save_secret,
            commands::keychain::secret_last_four,
            commands::keychain::delete_secret,
            commands::worktree::run_in_worktree,
            commands::completion::stream_completion,
            commands::session::load_session,
            commands::session::complete_first_launch,
            commands::session::create_workspace,
            commands::session::switch_workspace,
            commands::session::list_teams,
            commands::session::create_team,
            commands::session::create_agent,
            commands::session::get_agent,
            commands::session::update_agent,
            commands::session::delete_agent,
            commands::session::save_canvas_positions,
            commands::session::reset_canvas_positions,
            commands::session::list_agent_roles,
            commands::session::list_llm_providers,
            commands::session::detect_local_models,
            commands::session::list_agent_templates,
            commands::session::change_locale,
            commands::session::change_theme
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

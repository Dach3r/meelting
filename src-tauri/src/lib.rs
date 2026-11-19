mod catalogs;
mod commands;
mod domain;
mod repositories;
mod services;
mod system;

use crate::catalogs::llm;
use crate::domain::agent_run::Limits;
use crate::services::agent_run_engine::Engine;
use crate::services::agent_runs_expire;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let pool = tauri::async_runtime::block_on(repositories::database::open(
                &data_dir.join("meelting.db"),
            ))?;
            tauri::async_runtime::block_on(agent_runs_expire::run(&pool))?;
            let emit = commands::agent_runs::emitter(app.handle().clone());
            app.manage(Engine::new(
                pool.clone(),
                llm::providers(),
                Limits::default(),
                emit,
            ));
            app.manage(pool);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::worktree::run_in_worktree,
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
            commands::session::change_theme,
            commands::session::change_ui_scale,
            commands::session::change_loading_enabled,
            commands::rooms::list_rooms,
            commands::rooms::create_room,
            commands::rooms::get_room,
            commands::rooms::finish_room,
            commands::rooms::add_room_agents,
            commands::rooms::remove_room_agent,
            commands::agent_runs::send_intervention,
            commands::agent_runs::resolve_recipient,
            commands::agent_runs::check_agent_availability,
            commands::agent_runs::cancel_agent_run,
            commands::provider_keys::list_provider_keys,
            commands::provider_keys::save_provider_key,
            commands::provider_keys::check_provider_key,
            commands::provider_keys::delete_provider_key,
            commands::repositories::list_repositories,
            commands::repositories::register_repository,
            commands::repositories::unregister_repository,
            commands::repositories::list_repository_branches,
            commands::tasks::list_tasks,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::change_task_status,
            commands::tasks::delete_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

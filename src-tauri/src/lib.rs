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
use crate::services::file_copy::CopySelections;
use crate::services::repository_locks::RepositoryLocks;
use crate::services::review_registry::ReviewRegistry;
use crate::services::{worktree_purge, worktrees_reconcile};
use crate::system::storage;
use sqlx::SqlitePool;
use std::time::Duration;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let pool = tauri::async_runtime::block_on(repositories::database::open(
                &data_dir.join("meelting.db"),
            ))?;
            tauri::async_runtime::block_on(agent_runs_expire::run(&pool))?;
            let locks = RepositoryLocks::default();
            tauri::async_runtime::block_on(worktrees_reconcile::run(&pool, &locks))?;
            app.manage(locks.clone());
            app.manage(CopySelections::default());
            app.manage(ReviewRegistry::default());
            #[cfg(target_os = "macos")]
            app.manage(std::sync::Arc::new(services::voice::Voice::new(
                &data_dir.join("models"),
                commands::voice::playback_emitter(app.handle().clone()),
            )));
            let emit = commands::agent_runs::emitter(app.handle().clone());
            app.manage(Engine::new(
                pool.clone(),
                llm::providers(),
                Limits::default(),
                locks,
                emit,
            ));
            app.manage(pool);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let pool = handle.state::<SqlitePool>();
                    let locks = handle.state::<RepositoryLocks>();
                    let _ = worktree_purge::expired(&pool, &locks, storage::check).await;
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::session::load_session,
            commands::session::complete_first_launch,
            commands::session::create_workspace,
            commands::session::switch_workspace,
            commands::session::list_teams,
            commands::session::create_team,
            commands::session::delete_team,
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
            commands::session::change_time_format,
            commands::session::change_reduce_motion,
            commands::session::change_notification,
            commands::app_settings::get_open_at_login,
            commands::app_settings::change_open_at_login,
            commands::app_settings::open_data_folder,
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
            commands::agent_runs::stop_intervention,
            commands::agent_runs::get_agent_tools,
            commands::agent_runs::change_agent_tools,
            commands::voice::get_voice_status,
            #[cfg(target_os = "macos")]
            commands::voice::download_voice_model,
            #[cfg(target_os = "macos")]
            commands::voice::start_voice_capture,
            #[cfg(target_os = "macos")]
            commands::voice::finish_voice_capture,
            #[cfg(target_os = "macos")]
            commands::voice::cancel_voice_capture,
            #[cfg(target_os = "macos")]
            commands::voice::speak_message,
            #[cfg(target_os = "macos")]
            commands::voice::stop_speaking,
            commands::provider_keys::list_provider_keys,
            commands::provider_keys::save_provider_key,
            commands::provider_keys::check_provider_key,
            commands::provider_keys::delete_provider_key,
            commands::repositories::list_repositories,
            commands::repositories::register_repository,
            commands::repositories::unregister_repository,
            commands::repositories::list_repository_branches,
            commands::worktrees::get_worktrees_root,
            commands::worktrees::change_worktrees_root,
            commands::worktrees::prepare_task_environment,
            commands::worktrees::discard_task_environment,
            commands::worktrees::check_task_environment_removal,
            commands::worktrees::remove_task_environment,
            commands::worktrees::pick_task_environment_files,
            commands::worktrees::copy_task_environment_files,
            commands::worktrees::list_task_environments,
            commands::review::review_task_environment,
            commands::review::read_task_environment_diff,
            commands::review::list_task_environment_folder,
            commands::worktrees::trash_task_environments,
            commands::worktrees::restore_task_environments,
            commands::worktrees::purge_task_environments,
            commands::tasks::list_tasks,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::link_task_to_room,
            commands::tasks::change_task_status,
            commands::tasks::delete_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_webview_gets_only_the_core_permissions_and_no_file_or_shell_access() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();

        assert_eq!(capability["windows"], serde_json::json!(["main"]));
        assert_eq!(
            capability["permissions"],
            serde_json::json!(["core:default"])
        );
        let capabilities = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/capabilities"))
            .unwrap()
            .count();
        assert_eq!(capabilities, 1);
    }
}

mod agent_hooks;
mod agent_manager;
mod annotation_store;
mod clipboard;
mod confirmation_bridge;
mod core;
mod desktop_notify;
mod editor_host;
mod events;
mod git_engine;
mod home;
mod launch_env;
mod mcp_server;
mod migrate_identity;
mod mr_manager;
mod ops;
mod pty_trace;
mod task_manager;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Before anything spawns a child or opens a window.
    //
    // A desktop launch has none of the shell profile's PATH, so `glab`, `gh` and
    // `claude` are "not found" while a terminal finds them; and the identity
    // migration has to beat the webview to the data directory, which rules out
    // Tauri's `setup()` hook.
    launch_env::widen_path();
    if let Some((config_dir, data_dir)) = migrate_identity::linux_dirs() {
        migrate_identity::from_legacy_identity(&config_dir, &data_dir);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            pty_trace::init(&data_dir);
            let handle = app.handle().clone();

            // Spin up the async init inside Tauri's tokio runtime, then block the
            // main thread on a sync channel until it's done.  This guarantees that
            // all managed states are ready before the first IPC command arrives.
            let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();

            tauri::async_runtime::spawn(async move {
                let result = async_init(handle, data_dir).await;
                let _ = tx.send(result);
            });

            rx.recv()
                .expect("init channel closed unexpectedly")
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // task_manager
            task_manager::list_tasks,
            task_manager::open_task,
            task_manager::set_active_task,
            task_manager::open_explorer_session,
            task_manager::open_review_session,
            task_manager::rename_explorer,
            task_manager::discard_explorer,
            task_manager::pause_task,
            task_manager::sync_task,
            task_manager::set_task_repos,
            task_manager::get_config,
            task_manager::check_environment,
            task_manager::detect_database,
            task_manager::start_auth_session,
            task_manager::list_notion_users,
            task_manager::write_initial_config,
            task_manager::get_task_body_markdown,
            task_manager::get_task_schema,
            task_manager::list_relation_options,
            task_manager::get_task_properties,
            task_manager::update_task_property,
            task_manager::request_task_body_update,
            task_manager::create_task,
            task_manager::get_task_template_markdown,
            task_manager::get_task_time,
            task_manager::add_task_time,
            task_manager::log_task_hours,
            task_manager::set_font_size,
            task_manager::set_font_family,
            task_manager::list_fonts,
            task_manager::set_theme,
            task_manager::finish_task,
            task_manager::delete_task,
            // git_engine
            git_engine::register_repo,
            git_engine::list_main_repos,
            git_engine::clone_repo,
            git_engine::provision_worktrees,
            git_engine::provision_explorer_worktrees,
            git_engine::close_worktree,
            git_engine::remote_branch_exists,
            git_engine::get_task_diff_summary,
            git_engine::get_file_diff,
            git_engine::read_file_lines,
            git_engine::blame_file,
            git_engine::get_commit_diff,
            git_engine::get_commit_log,
            git_engine::get_worktree_status,
            git_engine::commit,
            git_engine::stage_file,
            git_engine::unstage_file,
            git_engine::stage_all,
            git_engine::unstage_all,
            git_engine::discard_file,
            git_engine::discard_all,
            git_engine::push,
            git_engine::pull,
            git_engine::rebase_on_main,
            git_engine::rebase_continue,
            git_engine::rebase_abort,
            git_engine::watch_task_worktrees,
            // home
            home::get_home_snapshot,
            // mcp_server
            mcp_server::get_mcp_endpoint,
            // agent_hooks
            agent_hooks::get_agent_activity,
            // mr_manager
            mr_manager::get_mr,
            mr_manager::create_mr,
            mr_manager::get_mr_threads,
            mr_manager::get_mr_ci,
            mr_manager::get_mr_details,
            mr_manager::reply_to_thread,
            mr_manager::resolve_mr_thread,
            mr_manager::approve_mr,
            mr_manager::edit_mr_text,
            mr_manager::post_mr_comment,
            mr_manager::list_review_mrs,
            // annotation_store
            annotation_store::create_annotation,
            annotation_store::resolve_annotation,
            annotation_store::get_annotations,
            annotation_store::delete_annotation,
            // agent_manager
            agent_manager::start_agent_session,
            agent_manager::start_terminal_session,
            agent_manager::stop_agent_session,
            agent_manager::resolve_confirmation,
            agent_manager::write_pty,
            pty_trace::trace_pty,
            pty_trace::pty_trace_on,
            agent_manager::resize_pty,
            // clipboard
            clipboard::copy_to_clipboard,
            clipboard::read_clipboard,
            desktop_notify::notify_desktop,
            // editor_host
            editor_host::list_files,
            editor_host::read_file,
            editor_host::open_file,
            editor_host::search_files,
            editor_host::update_open_file_state,
            editor_host::save_file,
            editor_host::create_file,
            editor_host::create_directory,
            editor_host::rename_path,
            editor_host::copy_path,
            editor_host::delete_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn async_init(handle: tauri::AppHandle, data_dir: std::path::PathBuf) -> Result<(), String> {
    // Config first: the worktree root and agent cwd resolve from it, and the
    // config dir is remembered here for every later save.
    let config_dir = handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("cannot get config dir: {e}"))?;
    crate::core::config::init(config_dir);

    let pool = crate::core::db::init(&data_dir)
        .await
        .map_err(|e| format!("DB init failed: {e}"))?;

    // Lets background work (git provisioning) report problems it can't return.
    events::set_app(handle.clone());

    let bridge = confirmation_bridge::Bridge::new(handle.clone());
    let git_state = git_engine::State::new();
    let editor_state = editor_host::State::new();
    let agent_state = agent_manager::State::new();
    let task_state = task_manager::State::new();
    let activity = agent_hooks::new_state();

    handle.manage(pool.clone());
    handle.manage(bridge.clone());
    handle.manage(git_state);
    handle.manage(editor_state.clone());
    handle.manage(agent_state);
    handle.manage(task_state.clone());
    handle.manage(activity.clone());


    // Re-emit any confirmations that survived a crash
    let pool_c = pool.clone();
    let handle_c = handle.clone();
    tokio::spawn(async move {
        confirmation_bridge::surface_pending(&pool_c, &handle_c).await;
    });

    // Start the MCP server (endpoint owned by `mcp_server`)
    let bridge_c = bridge;
    let task_c = task_state;
    let editor_c = editor_state;
    tokio::spawn(async move {
        if let Err(e) = mcp_server::start(bridge_c, pool, task_c, editor_c, activity).await {
            tracing::error!("MCP server error: {e}");
        }
    });

    Ok(())
}

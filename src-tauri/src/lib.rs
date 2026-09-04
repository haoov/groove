mod agent_hooks;
mod agent_manager;
mod annotation_store;
mod clipboard;
mod approvals;
mod core;
mod desktop_notify;
mod editor_host;
mod home;
mod launch_env;
mod mcp_server;
#[cfg(target_os = "linux")]
mod migrate_identity;
mod platform;
mod forge;
mod provider;
mod review;
mod skills;
mod task_manager;
mod worktrees;

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
    // XDG paths only; macOS uses ~/Library/Application Support.
    #[cfg(target_os = "linux")]
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
        // Closing the main window quits, or a detached agent window keeps the
        // process alive with nothing to drive it.
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
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
            task_manager::open_task,
            task_manager::set_active_task,
            task_manager::open_explorer_session,
            task_manager::open_review_session,
            task_manager::rename_explorer,
            task_manager::discard_explorer,
            task_manager::pause_task,
            task_manager::set_task_repos,
            task_manager::get_config,
            task_manager::check_environment,
            provider::detect_notion_database,
            task_manager::start_auth_session,
            task_manager::write_initial_config,
            task_manager::get_task_time,
            task_manager::get_activity_days,
            task_manager::add_task_time,
            task_manager::log_task_hours,
            task_manager::set_font_size,
            task_manager::set_font_family,
            task_manager::list_fonts,
            task_manager::set_theme,
            task_manager::set_suggest_actions,
            task_manager::finish_task,
            task_manager::delete_task,
            // tasks
            provider::list_tasks,
            provider::sync_task,
            provider::find_notion_user,
            provider::get_task_body_markdown,
            provider::get_task_schema,
            provider::list_relation_options,
            provider::get_task_properties,
            provider::update_task_property,
            provider::request_task_body_update,
            provider::preview_github,
            task_manager::set_task_source,
            // worktrees
            worktrees::register_repo,
            worktrees::list_main_repos,
            worktrees::clone_repo,
            worktrees::provision_worktrees,
            worktrees::default_branch_for_session,
            worktrees::close_worktree,
            worktrees::remote_branch_exists,
            worktrees::list_origin_branches,
            worktrees::get_worktree_status,
            worktrees::commit,
            worktrees::stage_file,
            worktrees::unstage_file,
            worktrees::stage_all,
            worktrees::unstage_all,
            worktrees::discard_file,
            worktrees::discard_all,
            worktrees::push,
            worktrees::pull,
            worktrees::rebase_on_main,
            worktrees::rebase_continue,
            worktrees::rebase_abort,
            worktrees::flush_git_caches,
            // review
            review::get_task_diff_summary,
            review::get_file_diff,
            review::read_file_lines,
            review::blame_file,
            review::get_commit_diff,
            review::get_commit_log,
            // home
            home::get_home_snapshot,
            // mcp_server
            mcp_server::get_mcp_endpoint,
            // agent_hooks
            agent_hooks::get_agent_activity,
            // forge
            forge::get_mr,
            forge::create_mr,
            forge::get_mr_threads,
            forge::get_mr_ci,
            forge::get_mr_details,
            forge::reply_to_thread,
            forge::resolve_mr_thread,
            forge::approve_mr,
            forge::edit_mr_text,
            forge::post_mr_comment,
            forge::list_review_mrs,
            // annotation_store
            annotation_store::create_annotation,
            annotation_store::update_annotation,
            annotation_store::resolve_annotation,
            annotation_store::get_annotations,
            annotation_store::delete_annotation,
            // agent_manager + core::pty
            agent_manager::start_agent_session,
            agent_manager::start_terminal_session,
            // skills
            skills::list_agent_skills,
            skills::read_agent_skill,
            skills::save_user_skill,
            skills::delete_user_skill,
            core::pty::stop_agent_session,
            core::pty::write_pty,
            core::pty::resize_pty,
            // approvals
            approvals::resolve_confirmation,
            // platform
            platform::platform,
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

    // After the config, which decides where the user plugin lives; before any
    // agent can launch and ask for its `--plugin-dir` arguments.
    if let Err(e) = skills::sync(&handle) {
        tracing::warn!("skills not synced: {e}");
    }

    let pool = crate::core::db::init(&data_dir)
        .await
        .map_err(|e| format!("DB init failed: {e}"))?;

    // Lets background work (git provisioning) report problems it can't return.
    core::events::set_app(handle.clone());

    let bridge = approvals::Bridge::new(handle.clone());
    let editor_state = editor_host::State::new();
    let task_state = task_manager::State::new();
    let activity = agent_hooks::new_state();

    handle.manage(pool.clone());
    handle.manage(bridge.clone());
    handle.manage(editor_state.clone());
    handle.manage(core::pty::Ptys::new());
    handle.manage(task_state.clone());
    handle.manage(activity.clone());


    // Re-emit any confirmations that survived a crash
    let pool_c = pool.clone();
    let handle_c = handle.clone();
    tokio::spawn(async move {
        approvals::surface_pending(&pool_c, &handle_c).await;
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

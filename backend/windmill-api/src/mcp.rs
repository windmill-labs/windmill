pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_scripts))
        .route("/list_search", get(list_search_scripts))
        .route("/create", post(create_script))
        .route("/create_snapshot", post(create_snapshot_script))
        .route("/archive/p/*path", post(archive_script_by_path))
        .route("/get/draft/*path", get(get_script_by_path_w_draft))
        .route("/get/p/*path", get(get_script_by_path))
        .route("/get_triggers_count/*path", get(get_triggers_count))
        .route("/list_tokens/*path", get(list_tokens))
        .route("/raw/p/*path", get(raw_script_by_path))
        .route("/raw_unpinned/p/*path", get(raw_script_by_path_unpinned))
        .route("/exists/p/*path", get(exists_script_by_path))
        .route("/archive/h/:hash", post(archive_script_by_hash))
        .route("/delete/h/:hash", post(delete_script_by_hash))
        .route("/delete/p/*path", post(delete_script_by_path))
        .route("/get/h/:hash", get(get_script_by_hash))
        .route("/raw/h/:hash", get(raw_script_by_hash))
        .route("/deployment_status/h/:hash", get(get_deployment_status))
        .route("/list_paths", get(list_paths))
        .route(
            "/toggle_workspace_error_handler/p/*path",
            post(toggle_workspace_error_handler),
        )
        .route("/history/p/*path", get(get_script_history))
        .route("/get_latest_version/*path", get(get_latest_version))
        .route(
            "/list_paths_from_workspace_runnable/*path",
            get(list_paths_from_workspace_runnable),
        )
        .route(
            "/history_update/h/:hash/p/*path",
            post(update_script_history),
        )
}

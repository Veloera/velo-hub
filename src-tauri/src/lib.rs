// Module declarations
mod commands;
mod config;
mod converter;
mod crypto;
mod db;
mod error;
mod http_server;
mod limiter;
mod logging;
mod models;
mod provider;
mod repository;
mod router;
mod session;
mod validation;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging with file rotation
    let log_config = logging::LogConfig::default();
    if let Err(e) = logging::init_logging(log_config) {
        eprintln!("Failed to initialize logging: {}", e);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize application state
            let app_handle = app.handle().clone();

            // Spawn HTTP server in background
            tauri::async_runtime::spawn(async move {
                if let Err(e) = http_server::start_server(app_handle).await {
                    tracing::error!("Failed to start HTTP server: {}", e);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_providers,
            commands::create_provider,
            commands::update_provider,
            commands::delete_provider,
            commands::test_provider_connection,
            commands::get_dashboard_metrics,
            commands::get_active_sessions,
            commands::get_circuit_breaker_status,
            commands::get_consumption_rankings,
            commands::export_config,
            commands::import_config,
            commands::get_model_pricing,
            commands::search_model_pricing,
            commands::sync_pricing_from_litellm,
            commands::get_session_history,
            commands::get_session_decision_chain,
            commands::get_global_config,
            commands::update_global_config,
            commands::get_model_redirects,
            commands::get_model_redirect,
            commands::create_model_redirect,
            commands::update_model_redirect,
            commands::delete_model_redirect,
            commands::toggle_model_redirect,
            commands::get_logs,
            commands::get_log_files,
            commands::get_log_directory,
            commands::update_log_level,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

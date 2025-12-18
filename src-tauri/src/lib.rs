use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{Manager, State};

mod config;
mod db;
mod server;
mod spider;
mod state;

use config::{AppConfig, ConfigManager};
use state::AppState;

#[derive(serde::Serialize)]
struct FrontendStats {
    service_req_count: usize,
    service_avg_time: f64,
    db_blocked_count: usize,
    spider_cache_count: usize,
    spider_queue_size: usize,
    spider_fail_count: usize,
    spider_req_avg_time: f64,
    uptime: u64,
    session_cleaned_count: usize,
    is_paused: bool,
    spider_total_received: usize,
    spider_actual_reqs: usize,
    server_status: i8, // 0: Init, 1: Running, 2: Failed
}

#[tauri::command]
fn get_stats(state: State<Arc<AppState>>) -> FrontendStats {
    let req_count = state.service_stats.req_count.load(Ordering::Relaxed);
    let req_time = state.service_stats.req_time_sum.load(Ordering::Relaxed);
    
    let spider_req_time = state.spider_stats.req_time_sum.load(Ordering::Relaxed);
    let spider_total_reqs = state.spider_stats.bv_cache_count.load(Ordering::Relaxed) + state.spider_stats.fail_count.load(Ordering::Relaxed);
    
    FrontendStats {
        service_req_count: req_count,
        service_avg_time: if req_count > 0 { req_time as f64 / req_count as f64 } else { 0.0 },
        db_blocked_count: state.db_stats.blocked_user_count.load(Ordering::Relaxed),
        spider_cache_count: state.spider_stats.bv_cache_count.load(Ordering::Relaxed),
        spider_queue_size: state.spider_stats.queue_size.load(Ordering::Relaxed),
        spider_fail_count: state.spider_stats.fail_count.load(Ordering::Relaxed),
        spider_req_avg_time: if spider_total_reqs > 0 { spider_req_time as f64 / spider_total_reqs as f64 } else { 0.0 },
        uptime: state.start_time.elapsed().as_secs(),
        session_cleaned_count: state.spider_stats.session_cleaned_count.load(Ordering::Relaxed),
        is_paused: state.spider_stats.is_paused.load(Ordering::Relaxed),
        spider_total_received: state.spider_stats.total_received_count.load(Ordering::Relaxed),
        spider_actual_reqs: state.spider_stats.actual_api_req_count.load(Ordering::Relaxed),
        server_status: state.server_status.load(Ordering::Relaxed),
    }
}

#[tauri::command]
fn get_app_config(state: State<Arc<ConfigManager>>) -> AppConfig {
    state.get_config()
}

#[tauri::command]
fn set_app_config(state: State<Arc<ConfigManager>>, config: AppConfig) -> Result<(), String> {
    state.set_config(config)
}

#[tauri::command]
fn toggle_spider_status(state: State<Arc<AppState>>) -> bool {
    let current = state.spider_stats.is_paused.load(Ordering::Relaxed);
    state.spider_stats.is_paused.store(!current, Ordering::Relaxed);
    !current
}

#[tauri::command]
fn set_always_on_top(window: tauri::Window, always_on_top: bool) -> Result<(), String> {
    window.set_always_on_top(always_on_top).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize DB
    let db_path = "./blocked_users.db";
    let conn = db::init_db(db_path).expect("Failed to init DB");
    
    // Initialize Config
    let config_manager = Arc::new(ConfigManager::new("./config.json"));

    // Initial stats load
    let blocked_count = db::get_blocked_count(&conn).unwrap_or(0);
    let cache_count = db::get_bv_cache_count(&conn).unwrap_or(0);

    let (tx, rx) = tokio::sync::mpsc::channel(1000);
    let app_state = Arc::new(AppState::new(conn, tx));
    
    app_state.db_stats.blocked_user_count.store(blocked_count, Ordering::Relaxed);
    app_state.spider_stats.bv_cache_count.store(cache_count, Ordering::Relaxed);

    let spider_state = app_state.clone();
    let server_state = app_state.clone();
    let cleaner_state = app_state.clone();
    let spider_config = config_manager.clone();
    let cleaner_config = config_manager.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .manage(config_manager)
        .setup(move |app| {
             if let Some(window) = app.get_webview_window("main") {
                 let _ = window.eval("document.addEventListener('contextmenu', e => e.preventDefault());");
             }

             // Spawn Spider
             tauri::async_runtime::spawn(async move {
                 spider::start_spider(spider_state, rx, spider_config).await;
             });

             // Spawn Cleaner
             tauri::async_runtime::spawn(async move {
                // Initial sleep to let app startup
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                
                loop {
                    let days = cleaner_config.get_config().cache_expiration_days;
                    if days > 0 {
                        let secs = (days * 24 * 3600) as i64;
                        // Use a block to drop the lock after operation
                        {
                            let conn = cleaner_state.db_conn.lock().await;
                            if let Ok(deleted) = db::clean_expired_cache(&*conn, secs) {
                                cleaner_state.spider_stats.session_cleaned_count.fetch_add(deleted, Ordering::Relaxed);
                                // Update stats
                                if let Ok(count) = db::get_bv_cache_count(&*conn) {
                                    cleaner_state.spider_stats.bv_cache_count.store(count, Ordering::Relaxed);
                                }
                            } else {
                                eprintln!("Failed to clean cache");
                            }
                        }
                    }
                    // Check every hour
                    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                }
             });

             // Start Server (it spawns its own thread)
             server::run_server(server_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_stats, get_app_config, set_app_config, toggle_spider_status, set_always_on_top])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

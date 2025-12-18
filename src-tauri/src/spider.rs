use crate::db;
use crate::state::AppState;
use crate::config::ConfigManager;
use reqwest::{Client, Proxy};
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use std::fs::{self, OpenOptions};
use std::io::Write;
use serde_json;
use chrono::Local;
use lazy_static::lazy_static;
use std::sync::Mutex;

#[derive(Deserialize, Debug)]
struct BilibiliApiResponse {
    code: i32,
    data: Option<BilibiliApiData>,
}

#[derive(Deserialize, Debug)]
struct BilibiliApiData {
    owner: Option<BilibiliOwner>,
}

#[derive(Deserialize, Debug)]
struct BilibiliOwner {
    mid: i64,
}

lazy_static! {
    static ref LOG_LOCK: Mutex<()> = Mutex::new(());
}

fn write_log(message: &str) {
    let _guard = LOG_LOCK.lock().unwrap();
    let log_dir = "./log";
    if let Err(_) = fs::create_dir_all(log_dir) {
        return;
    }

    let date = Local::now().format("%Y-%m-%d").to_string();
    let file_path = format!("{}/spider_{}.log", log_dir, date);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
    {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        if let Err(_) = writeln!(file, "[{}] {}", timestamp, message) {
            // 如果写入失败，静默处理避免递归错误
        }
    }
}

fn clean_old_logs() {
    let log_dir = "./log";
    if let Ok(entries) = fs::read_dir(log_dir) {
        let now = Local::now();
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("spider_") && file_name.ends_with(".log") {
                         // Parse date from filename: spider_2023-10-27.log
                         let date_part = &file_name[7..file_name.len()-4];
                         if let Ok(date) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                             let days_diff = (now.date_naive() - date).num_days();
                             if days_diff >= 7 {
                                 let _ = fs::remove_file(path);
                             }
                         }
                    }
                }
            }
        }
    }
}

pub async fn start_spider(state: Arc<AppState>, mut rx: mpsc::Receiver<String>, config: Arc<ConfigManager>) {
    // Clean old logs on startup
    clean_old_logs();

    // Initial setup
    let current_config = config.get_config();
    let mut current_proxy_url = current_config.proxy_url;
    let mut current_proxy_enabled = current_config.proxy_enabled;
    let mut client = build_client(&current_proxy_url, current_proxy_enabled);

    // Limit concurrent API requests to avoid IP bans while maintaining high throughput
    let semaphore = Arc::new(Semaphore::new(16));

    while let Some(bvid) = rx.recv().await {
        // Check for proxy config change
        let new_config = config.get_config();
        let new_proxy_url = new_config.proxy_url;
        let new_proxy_enabled = new_config.proxy_enabled;
        
        if new_proxy_url != current_proxy_url || new_proxy_enabled != current_proxy_enabled {
            write_log(&format!("Proxy config changed. Rebuilding client..."));
            client = build_client(&new_proxy_url, new_proxy_enabled);
            current_proxy_url = new_proxy_url;
            current_proxy_enabled = new_proxy_enabled;
        }
        
        let state_clone = state.clone();
        let client_clone = client.clone();
        let bvid_clone = bvid.clone();
        let sem_clone = semaphore.clone();

        tokio::spawn(async move {
            // Wait for a slot to perform the request
            let _permit = sem_clone.acquire().await.unwrap();

            // Wait if paused
            while state_clone.spider_stats.is_paused.load(Ordering::Relaxed) {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            //TODO: 似乎是不必要的
            // 1. Double check cache (DB read is fast)
            // {
            //     let conn = state_clone.db_conn.lock().await;
            //     if let Ok(Some(_)) = db::get_mid_by_bv(&conn, &bvid_clone) {
            //         // Already cached, just finish
            //         state_clone.spider_stats.queue_size.fetch_sub(1, Ordering::Relaxed);
            //         return;
            //     }
            // }

            // 2. Perform API Request
            state_clone.spider_stats.actual_api_req_count.fetch_add(1, Ordering::Relaxed);
            let start_time = Instant::now();
            let url = format!("https://api.bilibili.com/x/web-interface/view?bvid={}", bvid_clone);
            
            let mut success = false;
            // Retry logic could be added here, but keeping it simple for speed
            match client_clone.get(&url).send().await {
                Ok(resp) => {
                    // 获取响应文本用于日志记录
                    match resp.text().await {
                        Ok(text) => {                            
                            // 解析JSON
                            match serde_json::from_str::<BilibiliApiResponse>(&text) {
                                Ok(json) => {
                                    let duration = start_time.elapsed().as_millis() as u64;
                                    state_clone.spider_stats.req_time_sum.fetch_add(duration, Ordering::Relaxed);

                                    if json.code == 0 {
                                        if let Some(data) = json.data {
                                            if let Some(owner) = data.owner {
                                                let conn = state_clone.db_conn.lock().await;
                                                // Update cache
                                                if let Ok(_) = db::cache_bv_mid(&conn, &bvid_clone, owner.mid) {
                                                    state_clone.spider_stats.bv_cache_count.fetch_add(1, Ordering::Relaxed);
                                                    success = true;
                                                }
                                            }
                                        }
                                    } else {
                                        // Logic for known API errors (e.g., -404)
                                        write_log(&format!("API error for {}: code {}", bvid_clone, json.code));
                                    }
                                }
                                Err(e) => {
                                    write_log(&format!("JSON parse error for {}: {}", bvid_clone, e));
                                    write_log(&format!("Response for {}: {}", bvid_clone, text));
                                }
                            }
                        }
                        Err(e) => {
                            write_log(&format!("Failed to read response text for {}: {}", bvid_clone, e));
                        }
                    }
                }
                Err(e) => {
                    write_log(&format!("Network error for {}: {}", bvid_clone, e));
                }
            }

            if !success {
                 state_clone.spider_stats.fail_count.fetch_add(1, Ordering::Relaxed);
            }
            
            // Remove from pending set so it can be requested again later
            {
                let mut pending = state_clone.pending_bvs.lock().await;
                pending.remove(&bvid_clone);
            }

            // Mark task as completed
            state_clone.spider_stats.queue_size.fetch_sub(1, Ordering::Relaxed);
        });
    }
}

fn build_client(proxy_url: &Option<String>, enabled: bool) -> Client {
    let mut builder = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0")
        .pool_idle_timeout(std::time::Duration::from_secs(15))
        .pool_max_idle_per_host(16);

    if enabled {
        if let Some(url) = proxy_url {
            if !url.is_empty() {
                 match Proxy::all(url) {
                     Ok(proxy) => {
                         builder = builder.proxy(proxy);
                         write_log(&format!("Proxy set to: {}", url));
                     },
                     Err(e) => write_log(&format!("Invalid proxy url '{}': {}", url, e)),
                 }
            }
        }
    }

    builder.build().unwrap_or_else(|e| {
        write_log(&format!("Failed to build client: {}", e));
        Client::new()
    })
}

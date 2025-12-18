use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU64, AtomicUsize};
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::Connection;
use std::collections::HashSet;
use std::time::Instant;

pub struct ServiceStats {
    pub req_count: AtomicUsize,
    pub req_time_sum: AtomicU64, // milliseconds
}

pub struct DbStats {
    pub blocked_user_count: AtomicUsize,
}

pub struct SpiderStats {
    pub bv_cache_count: AtomicUsize,
    pub req_time_sum: AtomicU64,
    pub fail_count: AtomicUsize,
    pub queue_size: AtomicUsize,
    pub session_cleaned_count: AtomicUsize,
    pub is_paused: AtomicBool,
    pub total_received_count: AtomicUsize,
    pub actual_api_req_count: AtomicUsize,
}

pub struct AppState {
    pub db_conn: Arc<Mutex<Connection>>,
    pub service_stats: ServiceStats,
    pub db_stats: DbStats,
    pub spider_stats: SpiderStats,
    pub spider_queue: tokio::sync::mpsc::Sender<String>,
    pub pending_bvs: Mutex<HashSet<String>>,
    pub start_time: Instant,
    pub server_status: AtomicI8, // 0: Init, 1: Running, 2: Failed/Occupied
}

impl AppState {
    pub fn new(db_conn: Connection, spider_tx: tokio::sync::mpsc::Sender<String>) -> Self {
        Self {
            db_conn: Arc::new(Mutex::new(db_conn)),
            service_stats: ServiceStats {
                req_count: AtomicUsize::new(0),
                req_time_sum: AtomicU64::new(0),
            },
            db_stats: DbStats {
                blocked_user_count: AtomicUsize::new(0),
            },
            spider_stats: SpiderStats {
                bv_cache_count: AtomicUsize::new(0),
                req_time_sum: AtomicU64::new(0),
                fail_count: AtomicUsize::new(0),
                queue_size: AtomicUsize::new(0),
                session_cleaned_count: AtomicUsize::new(0),
                is_paused: AtomicBool::new(false),
                total_received_count: AtomicUsize::new(0),
                actual_api_req_count: AtomicUsize::new(0),
            },
            spider_queue: spider_tx,
            pending_bvs: Mutex::new(HashSet::new()),
            start_time: Instant::now(),
            server_status: AtomicI8::new(0),
        }
    }
}

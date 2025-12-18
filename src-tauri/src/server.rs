use actix_cors::Cors;
use actix_web::{rt, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use crate::db;
use crate::state::AppState;

#[derive(Deserialize)]
struct BlockForm {
    mid: String,
    username: Option<String>,
}

#[derive(Deserialize)]
struct RemoveForm {
    mid: String,
}

#[derive(Deserialize)]
struct IsBlockedBvsForm {
    bvs: String, // comma separated
}

#[derive(Serialize)]
struct IsBlockedBvsResponse {
    msg: String,
    mid: Vec<Option<i64>>,
    result: Vec<String>,
}

async fn add_user(form: web::Form<BlockForm>, state: web::Data<Arc<AppState>>) -> impl Responder {
    let mid_str = &form.mid;
    if !mid_str.chars().all(char::is_numeric) {
        return HttpResponse::Ok().body("ERR1");
    }

    let mid = match mid_str.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return HttpResponse::Ok().body("ERR1"),
    };

    let conn = state.db_conn.lock().await;
    match db::add_user(&conn, mid, form.username.as_deref()) {
        Ok(true) => {
            state
                .db_stats
                .blocked_user_count
                .fetch_add(1, Ordering::Relaxed);
            HttpResponse::Ok().body("OK")
        }
        Ok(false) => HttpResponse::Ok().body("ERR2"),
        Err(_) => HttpResponse::Ok().body("ERR2"),
    }
}

async fn remove_user(
    form: web::Form<RemoveForm>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let mid_str = &form.mid;
    if !mid_str.chars().all(char::is_numeric) {
        return HttpResponse::Ok().body("ERR1");
    }

    let mid = match mid_str.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return HttpResponse::Ok().body("ERR1"),
    };

    let conn = state.db_conn.lock().await;
    match db::remove_user(&conn, mid) {
        Ok(true) => {
            state
                .db_stats
                .blocked_user_count
                .fetch_sub(1, Ordering::Relaxed);
            HttpResponse::Ok().body("OK")
        }
        Ok(false) => HttpResponse::Ok().body("ERR2"),
        Err(_) => HttpResponse::Ok().body("ERR2"),
    }
}

async fn is_user_exist(
    query: web::Query<RemoveForm>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let start = Instant::now();
    let mid_str = &query.mid;
    if !mid_str.chars().all(char::is_numeric) {
        return HttpResponse::Ok().body("ERR1");
    }

    let mid = match mid_str.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return HttpResponse::Ok().body("ERR1"),
    };

    let conn = state.db_conn.lock().await;
    let res = match db::is_user_exist(&conn, mid) {
        Ok(true) => HttpResponse::Ok().body("True"),
        Ok(false) => HttpResponse::Ok().body("False"),
        Err(_) => HttpResponse::Ok().body("ERR2"),
    };

    state
        .service_stats
        .req_count
        .fetch_add(1, Ordering::Relaxed);
    state
        .service_stats
        .req_time_sum
        .fetch_add(start.elapsed().as_millis() as u64, Ordering::Relaxed);
    res
}

#[derive(Deserialize)]
struct IsExistSForm {
    mids: String,
}

async fn is_user_exist_s_impl(
    form: web::Form<IsExistSForm>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let start = Instant::now();
    let mids_str = &form.mids;
    let mids: Vec<&str> = mids_str.split(',').collect();

    let mut results = Vec::new();
    let conn = state.db_conn.lock().await;

    for mid_str in mids {
        if !mid_str.chars().all(char::is_numeric) {
            results.push("ERR1".to_string());
            continue;
        }
        match mid_str.parse::<i64>() {
            Ok(mid) => match db::is_user_exist(&conn, mid) {
                Ok(true) => results.push("True".to_string()),
                Ok(false) => results.push("False".to_string()),
                Err(_) => results.push("ERR2".to_string()),
            },
            Err(_) => results.push("ERR1".to_string()),
        }
    }

    state
        .service_stats
        .req_count
        .fetch_add(1, Ordering::Relaxed);
    state
        .service_stats
        .req_time_sum
        .fetch_add(start.elapsed().as_millis() as u64, Ordering::Relaxed);
    HttpResponse::Ok().json(results)
}

async fn is_blocked_bvs(
    form: web::Form<IsBlockedBvsForm>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let start = Instant::now();
    let bvs: Vec<&str> = form.bvs.split(',').collect();

    let mut mids = Vec::new();
    let mut results = Vec::new();

    let conn = state.db_conn.lock().await;

    state.spider_stats.total_received_count.fetch_add(bvs.len(), Ordering::Relaxed);
    for bv in bvs {
        match db::get_mid_by_bv(&conn, bv) {
            Ok(Some(mid)) => {
                mids.push(Some(mid));
                match db::is_user_exist(&conn, mid) {
                    Ok(true) => results.push("True".to_string()),
                    Ok(false) => results.push("False".to_string()),
                    Err(_) => results.push("ERR2".to_string()),
                }
            }
            Ok(None) => {
                mids.push(None);
                results.push("None".to_string());

                // Deduplication logic
                let mut pending = state.pending_bvs.lock().await;
                if !pending.contains(bv) {
                    pending.insert(bv.to_string());
                    // Only queue if not already pending
                    let _ = state.spider_queue.send(bv.to_string()).await;
                    state
                        .spider_stats
                        .queue_size
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
            Err(_) => {
                mids.push(None);
                results.push("ERR2".to_string());
            }
        }
    }

    state
        .service_stats
        .req_count
        .fetch_add(1, Ordering::Relaxed);
    state
        .service_stats
        .req_time_sum
        .fetch_add(start.elapsed().as_millis() as u64, Ordering::Relaxed);

    HttpResponse::Ok().json(IsBlockedBvsResponse {
        msg: "OK".to_string(),
        mid: mids,
        result: results,
    })
}

async fn is_alive() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

pub fn run_server(state: Arc<AppState>) {
    std::thread::spawn(move || {
        let sys = rt::System::new();

        sys.block_on(async move {
            let data = web::Data::new(state.clone());

            let server_factory = HttpServer::new(move || {
                App::new()
                    .wrap(Cors::permissive())
                    .app_data(data.clone())
                    .route("/block", web::post().to(add_user))
                    .route("/remove", web::post().to(remove_user))
                    .route("/isExist", web::get().to(is_user_exist))
                    .route("/isExistS", web::post().to(is_user_exist_s_impl))
                    .route("/isBlockedBVS", web::post().to(is_blocked_bvs))
                    .route("/ok", web::get().to(is_alive))
            });

            match server_factory.bind(("127.0.0.1", 22332)) {
                Ok(server) => {
                    // Port bound successfully

                    state.server_status.store(1, Ordering::Relaxed);

                    if let Err(e) = server.run().await {
                        eprintln!("Server error: {}", e);

                        // If run fails after bind (rare, but possible)

                        state.server_status.store(2, Ordering::Relaxed);
                    }
                }

                Err(e) => {
                    eprintln!("Can not bind to port 22332: {}", e);

                    state.server_status.store(2, Ordering::Relaxed);
                }
            }
        });
    });
}

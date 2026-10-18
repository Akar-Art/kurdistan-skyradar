//! Kurdistan SkyRadar - High-Performance Rust Telemetry Server
//!
//! Aggregates real-time ADS-B and FlightRadar24 flight feeds,
//! geofences them with the Kurdistan Atlas, and streams telemetry.

mod feeds;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::{header, Method};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use radar_core::atlas::{Atlas, BUILTIN_ATLAS_JSON};
use radar_core::db::AIRPORTS;
use radar_core::ingest::build_snapshot;
use radar_core::model::Snapshot;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

struct AppState {
    atlas: Atlas,
    snapshot: RwLock<Snapshot>,
    trails: RwLock<HashMap<String, Vec<[f32; 2]>>>,
}

fn now_epoch_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs_f64()
}

#[tokio::main]
async fn main() {
    println!("------------------------------------------------------------");
    println!("   KURDISTAN SKYRADAR - RUST TELEMETRY ENGINE");
    println!("------------------------------------------------------------");

    let atlas = Atlas::builtin();
    println!(
        "Atlas initialized with {} regions ({} points) and {} places.",
        atlas.regions.len(),
        atlas.point_count(),
        atlas.places.len()
    );

    let state = Arc::new(AppState {
        atlas,
        snapshot: RwLock::new(Snapshot::default()),
        trails: RwLock::new(HashMap::new()),
    });

    let poller_state = Arc::clone(&state);
    tokio::spawn(async move {
        background_worker(poller_state).await;
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    let dist_dirs = [
        PathBuf::from("crates/radar-app/dist"),
        PathBuf::from("dist"),
        PathBuf::from("../radar-app/dist"),
    ];
    let mut serve_dir = None;
    for d in &dist_dirs {
        if d.is_dir() {
            serve_dir = Some(d.clone());
            println!("Serving WebAssembly frontend from: {}", d.display());
            break;
        }
    }

    let mut app = Router::new()
        .route("/api/flights", get(get_snapshot))
        .route("/api/snapshot", get(get_snapshot))
        .route("/api/boundaries", get(get_boundaries))
        .route("/api/airports", get(get_airports))
        .layer(cors);

    if let Some(dir) = serve_dir {
        app = app.fallback_service(ServeDir::new(dir));
    }

    let app = app.with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Kurdistan SkyRadar Server listening on http://localhost:8080");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind port 8080");

    axum::serve(listener, app).await.unwrap();
}

async fn get_snapshot(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let snap = state.snapshot.read().await;
    (
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&*snap).unwrap_or_default(),
    )
}

async fn get_boundaries() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        BUILTIN_ATLAS_JSON,
    )
}

async fn get_airports() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&AIRPORTS).unwrap_or_default(),
    )
}

async fn background_worker(state: Arc<AppState>) {
    let client = feeds::http_client();
    println!("Background flight ingest worker started.");

    loop {
        let now = now_epoch_secs();
        let (fr24_res, adsb_res) = tokio::join!(
            feeds::fetch_fr24(&client, now),
            feeds::fetch_adsb(&client, now)
        );

        let mut trails_guard = state.trails.write().await;
        let snap = build_snapshot(&state.atlas, fr24_res, adsb_res, &mut trails_guard, now);
        drop(trails_guard);

        *state.snapshot.write().await = snap;

        tokio::time::sleep(Duration::from_millis(2800)).await;
    }
}

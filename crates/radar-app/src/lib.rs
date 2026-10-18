//! Kurdistan SkyRadar — Airspace Command & Control (C2) System
//!
//! 100% Rust standalone PC application and WebAssembly client for real-time
//! airspace monitoring across all four parts of Kurdistan (Bakur, Başûr,
//! Rojava, Rojhilat) and the Kurdistan Region (`Herêma Kurdistanê`).

use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use egui::{vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use radar_core::atlas::{Atlas, PlaceCategory};
use radar_core::db::AIRPORTS;
use radar_core::geo::{angle_diff, cardinal, dead_reckon};
use radar_core::model::{Flight, Snapshot};
use radar_core::KURDISTAN_CENTER;
use walkers::sources::{Attribution, TileSource};
use walkers::{lat_lon, HttpTiles, Map, MapMemory, Plugin, Position, Projector, TileId};

#[derive(Clone)]
pub struct EsriDark;

impl TileSource for EsriDark {
    fn tile_url(&self, tile_id: TileId) -> String {
        format!(
            "https://services.arcgisonline.com/arcgis/rest/services/Canvas/World_Dark_Gray_Base/MapServer/tile/{}/{}/{}",
            tile_id.zoom, tile_id.y, tile_id.x
        )
    }

    fn attribution(&self) -> Attribution {
        Attribution {
            text: "Kurdistan Atlas (atlas.kurd.sh) · Esri · OpenStreetMap",
            url: "https://atlas.kurd.sh",
            logo_light: None,
            logo_dark: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    All,
    GreaterKurdistan,
    Kri,
    MilGov,
    Erbil,
    Suly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKind {
    KurdistanFirst,
    AltitudeDesc,
    SpeedDesc,
    CallsignAsc,
}

/// Per-aircraft interpolated state for zero-jitter 60+ FPS motion.
#[derive(Debug, Clone)]
struct VisualTrack {
    display_lat: f64,
    display_lon: f64,
    display_heading: f32,
    target_lat: f64,
    target_lon: f64,
    target_heading: f32,
    speed_kts: f32,
    on_ground: bool,
}

pub struct RadarApp {
    map_memory: MapMemory,
    tiles: HttpTiles,
    atlas: Arc<Atlas>,
    snapshot: Option<Snapshot>,
    shared_snapshot: Arc<Mutex<Option<Snapshot>>>,
    last_snapshot_ts: f64,
    #[allow(dead_code)]
    last_poll_s: f64,
    #[allow(dead_code)]
    polling: Arc<AtomicBool>,
    tracks: HashMap<String, VisualTrack>,

    selected_hex: Option<String>,
    follow_hex: Option<String>,
    filter: FilterKind,
    sort_by: SortKind,
    search_query: String,

    // Map layer toggles
    show_borders: bool,
    show_disputed: bool,
    show_provinces: bool,
    show_luristan: bool,
    show_places: bool,
    show_airports: bool,
    show_trails: bool,
    show_vectors: bool,
    sidebar_open: bool,
}

impl RadarApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgba_unmultiplied(9, 13, 20, 244);
        visuals.window_fill = Color32::from_rgba_unmultiplied(11, 16, 25, 248);
        visuals.faint_bg_color = Color32::from_rgba_unmultiplied(16, 23, 36, 220);
        visuals.extreme_bg_color = Color32::from_rgb(6, 9, 14);
        visuals.widgets.noninteractive.bg_stroke =
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(51, 65, 85, 110));
        cc.egui_ctx.set_visuals(visuals);

        let tiles = HttpTiles::new(EsriDark, cc.egui_ctx.clone());
        let mut map_memory = MapMemory::default();
        map_memory.set_zoom(6.6).ok();
        map_memory.center_at(lat_lon(KURDISTAN_CENTER.0, KURDISTAN_CENTER.1));

        let atlas = Arc::new(Atlas::builtin());
        let shared_snapshot: Arc<Mutex<Option<Snapshot>>> = Arc::new(Mutex::new(None));

        // On native PC, start the built-in background telemetry engine so
        // KurdistanSkyRadar.exe operates 100% standalone without an external server.
        #[cfg(not(target_arch = "wasm32"))]
        spawn_native_telemetry_worker(
            Arc::clone(&atlas),
            Arc::clone(&shared_snapshot),
            cc.egui_ctx.clone(),
        );

        Self {
            map_memory,
            tiles,
            atlas,
            snapshot: None,
            shared_snapshot,
            last_snapshot_ts: 0.0,
            last_poll_s: -10.0,
            polling: Arc::new(AtomicBool::new(false)),
            tracks: HashMap::new(),
            selected_hex: None,
            follow_hex: None,
            filter: FilterKind::All,
            sort_by: SortKind::KurdistanFirst,
            search_query: String::new(),
            show_borders: true,
            show_disputed: true,
            show_provinces: false,
            show_luristan: false,
            show_places: true,
            show_airports: true,
            show_trails: true,
            show_vectors: true,
            sidebar_open: true,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn trigger_wasm_poll(&mut self, ctx: &egui::Context, now: f64) {
        if self.polling.load(Ordering::Relaxed) {
            return;
        }
        self.polling.store(true, Ordering::Relaxed);
        let polling_flag = Arc::clone(&self.polling);
        let shared = Arc::clone(&self.shared_snapshot);
        let ctx_clone = ctx.clone();

        let request = ehttp::Request::get("/api/flights");
        ehttp::fetch(request, move |result| {
            polling_flag.store(false, Ordering::Relaxed);
            if let Ok(response) = result {
                if response.status == 200 {
                    if let Ok(snap) = serde_json::from_slice::<Snapshot>(&response.bytes) {
                        if let Ok(mut guard) = shared.lock() {
                            *guard = Some(snap);
                        }
                        ctx_clone.request_repaint();
                    }
                }
            }
        });
        self.last_poll_s = now;
    }

    fn ingest_snapshot(&mut self, snap: Snapshot) {
        if snap.timestamp <= self.last_snapshot_ts {
            return;
        }
        self.last_snapshot_ts = snap.timestamp;

        for f in &snap.flights {
            let age = f.pos_age_s.clamp(0.0, 15.0) as f64;
            let (t_lat, t_lon) = if f.on_ground {
                (f.lat, f.lon)
            } else {
                dead_reckon(f.lat, f.lon, f.heading as f64, f.speed_kts as f64, age)
            };

            match self.tracks.get_mut(&f.hex) {
                Some(tr) => {
                    let jump_deg = (tr.display_lat - t_lat).hypot(tr.display_lon - t_lon);
                    if jump_deg > 0.45 {
                        // Large discontinuity: snap directly
                        tr.display_lat = t_lat;
                        tr.display_lon = t_lon;
                        tr.display_heading = f.heading;
                    }
                    tr.target_lat = t_lat;
                    tr.target_lon = t_lon;
                    tr.target_heading = f.heading;
                    tr.speed_kts = f.speed_kts;
                    tr.on_ground = f.on_ground;
                }
                None => {
                    self.tracks.insert(
                        f.hex.clone(),
                        VisualTrack {
                            display_lat: t_lat,
                            display_lon: t_lon,
                            display_heading: f.heading,
                            target_lat: t_lat,
                            target_lon: t_lon,
                            target_heading: f.heading,
                            speed_kts: f.speed_kts,
                            on_ground: f.on_ground,
                        },
                    );
                }
            }
        }

        let active: std::collections::HashSet<&str> =
            snap.flights.iter().map(|f| f.hex.as_str()).collect();
        self.tracks.retain(|k, _| active.contains(k.as_str()));
        self.snapshot = Some(snap);
    }

    fn step_visual_tracks(&mut self, dt_s: f32) {
        let dt = dt_s.clamp(0.001, 0.1) as f64;
        let pos_blend = 1.0 - (-dt * 3.8).exp();
        let hdg_blend = 1.0 - (-dt * 6.5).exp();

        for tr in self.tracks.values_mut() {
            if !tr.on_ground && tr.speed_kts > 8.0 {
                let (nt_lat, nt_lon) = dead_reckon(
                    tr.target_lat,
                    tr.target_lon,
                    tr.target_heading as f64,
                    tr.speed_kts as f64,
                    dt,
                );
                tr.target_lat = nt_lat;
                tr.target_lon = nt_lon;

                let (nd_lat, nd_lon) = dead_reckon(
                    tr.display_lat,
                    tr.display_lon,
                    tr.display_heading as f64,
                    tr.speed_kts as f64,
                    dt,
                );
                tr.display_lat = nd_lat;
                tr.display_lon = nd_lon;
            }

            tr.display_lat += (tr.target_lat - tr.display_lat) * pos_blend;
            tr.display_lon += (tr.target_lon - tr.display_lon) * pos_blend;

            let d_hdg = angle_diff(tr.display_heading as f64, tr.target_heading as f64);
            tr.display_heading =
                (tr.display_heading as f64 + d_hdg * hdg_blend).rem_euclid(360.0) as f32;
        }
    }

    fn jump_to_sector(&mut self, lat: f64, lon: f64, zoom: f64) {
        self.follow_hex = None;
        self.map_memory.center_at(lat_lon(lat, lon));
        self.map_memory.set_zoom(zoom).ok();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_native_telemetry_worker(
    atlas: Arc<Atlas>,
    shared_snapshot: Arc<Mutex<Option<Snapshot>>>,
    egui_ctx: egui::Context,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime for telemetry worker");

        rt.block_on(async move {
            let client = reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) KurdistanSkyRadarC2/0.3")
                .timeout(std::time::Duration::from_secs(6))
                .build()
                .expect("Failed to create HTTP client");

            let mut trails: HashMap<String, Vec<[f32; 2]>> = HashMap::new();

            loop {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64();

                let fr24_fut = async {
                    let url = radar_core::ingest::fr24_feed_url();
                    let text = client
                        .get(url)
                        .send()
                        .await
                        .ok()?
                        .error_for_status()
                        .ok()?
                        .text()
                        .await
                        .ok()?;
                    radar_core::ingest::parse_fr24_json(&text, now)
                };

                let adsb_fut = async {
                    for url in radar_core::ingest::adsb_feed_urls() {
                        let Ok(resp) = client.get(&url).send().await else {
                            continue;
                        };
                        let Ok(text) = resp.text().await else {
                            continue;
                        };
                        if let Some(list) = radar_core::ingest::parse_adsb_json(&text, now) {
                            return Some(list);
                        }
                    }
                    None
                };

                let (fr24_res, adsb_res) = tokio::join!(fr24_fut, adsb_fut);
                let snap =
                    radar_core::ingest::build_snapshot(&atlas, fr24_res, adsb_res, &mut trails, now);

                if let Ok(mut guard) = shared_snapshot.lock() {
                    *guard = Some(snap);
                }
                egui_ctx.request_repaint();

                tokio::time::sleep(std::time::Duration::from_millis(2600)).await;
            }
        });
    });
}

impl eframe::App for RadarApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let (_now, dt, esc_pressed) = ctx.input(|i| {
            (
                i.time,
                i.stable_dt,
                i.key_pressed(egui::Key::Escape),
            )
        });

        if esc_pressed {
            self.selected_hex = None;
            self.follow_hex = None;
        }

        // Check for plane click from map overlay
        if let Some(hex) =
            ui.data_mut(|d| d.remove_temp::<String>(egui::Id::new("clicked_flight_hex")))
        {
            if hex.is_empty() {
                self.selected_hex = None;
                self.follow_hex = None;
            } else {
                self.selected_hex = Some(hex);
            }
        }

        // Pull latest snapshot from background worker / WASM poller
        let incoming = self
            .shared_snapshot
            .lock()
            .ok()
            .and_then(|mut g| g.take());
        if let Some(snap) = incoming {
            self.ingest_snapshot(snap);
        }

        #[cfg(target_arch = "wasm32")]
        if _now - self.last_poll_s > 3.0 {
            self.trigger_wasm_poll(&ctx, _now);
        }

        // Advance all visual tracks smoothly by frame delta time
        self.step_visual_tracks(dt);

        // Follow camera lock if active
        if let Some(ref hex) = self.follow_hex {
            if let Some(tr) = self.tracks.get(hex) {
                self.map_memory
                    .center_at(lat_lon(tr.display_lat, tr.display_lon));
            }
        }

        // ── 1. TOP COMMAND BAR (42px Tactical Header) ───────────────────────────
        egui::Panel::top("c2_command_bar")
            .exact_size(42.0)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 0.0);

                    // Panel toggle button
                    let panel_label = if self.sidebar_open { "◀ CONSOLE" } else { "▶ CONSOLE" };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(panel_label)
                                    .monospace()
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::from_rgb(226, 232, 240)),
                            )
                            .fill(Color32::from_rgb(22, 31, 48)),
                        )
                        .clicked()
                    {
                        self.sidebar_open = !self.sidebar_open;
                    }

                    // Official C2 Title
                    ui.label(
                        egui::RichText::new("KURDISTAN")
                            .color(Color32::from_rgb(251, 191, 36))
                            .strong()
                            .size(15.0),
                    );
                    ui.label(
                        egui::RichText::new("SKYRADAR C2")
                            .color(Color32::WHITE)
                            .strong()
                            .size(15.0),
                    );

                    // Live Feed Status Badge
                    let (feed_ok, fr_ok, adsb_ok) = self
                        .snapshot
                        .as_ref()
                        .map(|s| (s.sources.fr24 || s.sources.adsb, s.sources.fr24, s.sources.adsb))
                        .unwrap_or((false, false, false));
                    let status_col = if feed_ok {
                        Color32::from_rgb(16, 185, 129)
                    } else {
                        Color32::from_rgb(245, 158, 11)
                    };
                    let status_txt = match (fr_ok, adsb_ok) {
                        (true, true) => "● LIVE HYBRID",
                        (true, false) => "● LIVE SAT/MLAT",
                        (false, true) => "● LIVE ADS-B",
                        (false, false) => "◌ SYNCING FEEDS",
                    };
                    ui.label(
                        egui::RichText::new(status_txt)
                            .color(status_col)
                            .monospace()
                            .strong()
                            .size(11.0),
                    );

                    ui.separator();

                    // Tactical Airspace Counters
                    let total = self.snapshot.as_ref().map(|s| s.flights.len()).unwrap_or(0);
                    let greater = self.snapshot.as_ref().map(|s| s.greater_count()).unwrap_or(0);
                    let kri = self.snapshot.as_ref().map(|s| s.kri_count()).unwrap_or(0);
                    let mil = self.snapshot.as_ref().map(|s| s.military_count()).unwrap_or(0);

                    tactical_stat_pill(ui, "SECTOR", &total.to_string(), Color32::from_rgb(148, 163, 184));
                    tactical_stat_pill(ui, "KURDISTAN", &greater.to_string(), Color32::from_rgb(56, 189, 248));
                    tactical_stat_pill(ui, "KRI", &kri.to_string(), Color32::from_rgb(251, 191, 36));
                    if mil > 0 {
                        tactical_stat_pill(ui, "MIL/GOV", &mil.to_string(), Color32::from_rgb(249, 115, 22));
                    }

                    ui.separator();

                    // Quick Sector Camera Jump Buttons
                    ui.label(
                        egui::RichText::new("SECTOR:")
                            .size(10.0)
                            .monospace()
                            .color(Color32::from_rgb(100, 116, 139)),
                    );
                    if sector_btn(ui, "ALL").clicked() {
                        self.jump_to_sector(36.9, 43.3, 6.4);
                    }
                    if sector_btn(ui, "BAŞÛR / KRI").clicked() {
                        self.jump_to_sector(36.05, 44.15, 7.8);
                    }
                    if sector_btn(ui, "BAKUR").clicked() {
                        self.jump_to_sector(38.45, 41.20, 7.2);
                    }
                    if sector_btn(ui, "ROJAVA").clicked() {
                        self.jump_to_sector(36.75, 40.10, 7.6);
                    }
                    if sector_btn(ui, "ROJHILAT").clicked() {
                        self.jump_to_sector(35.75, 46.40, 7.4);
                    }

                    // Right-aligned UTC & Hewlêr Clocks
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let epoch = self
                            .snapshot
                            .as_ref()
                            .map(|s| s.timestamp as u64)
                            .filter(|&t| t > 0)
                            .unwrap_or_else(current_unix_secs);
                        let (utc_h, utc_m, utc_s) = hms_from_epoch(epoch, 0);
                        let (ebl_h, ebl_m, _) = hms_from_epoch(epoch, 3 * 3600);

                        ui.label(
                            egui::RichText::new(format!(
                                "HEWLÊR {:02}:{:02}  |  {:02}:{:02}:{:02}Z",
                                ebl_h, ebl_m, utc_h, utc_m, utc_s
                            ))
                            .monospace()
                            .size(11.5)
                            .strong()
                            .color(Color32::from_rgb(203, 213, 225)),
                        );
                    });
                });
            });

        // ── 2. SINGLE LEFT TACTICAL CONSOLE (330px) ─────────────────────────────
        if self.sidebar_open {
            egui::Panel::left("tactical_console")
                .exact_size(336.0)
                .resizable(false)
                .show(ui, |ui| {
                    ui.add_space(6.0);

                    // Map Layer Toggles Strip
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(14, 20, 32))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(30, 41, 59)))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("MAP OVERLAYS & BOUNDARIES")
                                    .size(10.0)
                                    .monospace()
                                    .strong()
                                    .color(Color32::from_rgb(148, 163, 184)),
                            );
                            ui.add_space(3.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                                layer_chip(ui, &mut self.show_borders, "Kurdistan Borders");
                                layer_chip(ui, &mut self.show_disputed, "Disputed (Art.140)");
                                layer_chip(ui, &mut self.show_places, "Cities & Peaks");
                                layer_chip(ui, &mut self.show_airports, "Airports");
                                layer_chip(ui, &mut self.show_trails, "Trails");
                                layer_chip(ui, &mut self.show_vectors, "60s Vector");
                                layer_chip(ui, &mut self.show_provinces, "Provinces");
                                layer_chip(ui, &mut self.show_luristan, "Luristan");
                            });
                        });

                    ui.add_space(6.0);

                    // Integrated Selected Target Dossier (replaces the bloated right drawer!)
                    if let Some(ref hex) = self.selected_hex.clone() {
                        if let Some(f) = self
                            .snapshot
                            .as_ref()
                            .and_then(|s| s.flights.iter().find(|fl| &fl.hex == hex))
                            .cloned()
                        {
                            self.render_target_dossier(ui, &f);
                            ui.add_space(6.0);
                        }
                    }

                    // Search & Filter Controls
                    ui.horizontal(|ui| {
                        let response = ui.add_sized(
                            [ui.available_width() - if self.search_query.is_empty() { 0.0 } else { 28.0 }, 24.0],
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text("Search callsign, hex, reg, airport, city..."),
                        );
                        let _ = response;
                        if !self.search_query.is_empty() && ui.small_button("✕").clicked() {
                            self.search_query.clear();
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                        ui.selectable_value(&mut self.filter, FilterKind::All, "ALL");
                        ui.selectable_value(&mut self.filter, FilterKind::GreaterKurdistan, "KURDISTAN");
                        ui.selectable_value(&mut self.filter, FilterKind::Kri, "KRI");
                        ui.selectable_value(&mut self.filter, FilterKind::MilGov, "MIL/GOV");
                        ui.selectable_value(&mut self.filter, FilterKind::Erbil, "HEWLÊR");
                        ui.selectable_value(&mut self.filter, FilterKind::Suly, "SILÊMANÎ");
                    });

                    ui.add_space(4.0);

                    // Filter & sort flights
                    let mut flights: Vec<&Flight> = match &self.snapshot {
                        Some(snap) => snap
                            .flights
                            .iter()
                            .filter(|f| match self.filter {
                                FilterKind::All => true,
                                FilterKind::GreaterKurdistan => f.in_greater,
                                FilterKind::Kri => f.in_kri,
                                FilterKind::MilGov => f.is_military || f.is_emergency(),
                                FilterKind::Erbil => f.origin == "EBL" || f.dest == "EBL",
                                FilterKind::Suly => f.origin == "ISU" || f.dest == "ISU",
                            })
                            .filter(|f| {
                                if self.search_query.trim().is_empty() {
                                    return true;
                                }
                                let q = self.search_query.trim().to_lowercase();
                                f.callsign.to_lowercase().contains(&q)
                                    || f.hex.to_lowercase().contains(&q)
                                    || f.registration.to_lowercase().contains(&q)
                                    || f.airline.to_lowercase().contains(&q)
                                    || f.model_code.to_lowercase().contains(&q)
                                    || f.model_name.to_lowercase().contains(&q)
                                    || f.origin.to_lowercase().contains(&q)
                                    || f.dest.to_lowercase().contains(&q)
                                    || f.origin_city.to_lowercase().contains(&q)
                                    || f.dest_city.to_lowercase().contains(&q)
                                    || f.region
                                        .as_deref()
                                        .unwrap_or_default()
                                        .to_lowercase()
                                        .contains(&q)
                            })
                            .collect(),
                        None => Vec::new(),
                    };

                    match self.sort_by {
                        SortKind::KurdistanFirst => {
                            flights.sort_by(|a, b| {
                                b.is_emergency()
                                    .cmp(&a.is_emergency())
                                    .then_with(|| b.is_military.cmp(&a.is_military))
                                    .then_with(|| b.in_kri.cmp(&a.in_kri))
                                    .then_with(|| b.in_greater.cmp(&a.in_greater))
                                    .then_with(|| a.dist_erbil_nm.total_cmp(&b.dist_erbil_nm))
                            });
                        }
                        SortKind::AltitudeDesc => {
                            flights.sort_by(|a, b| b.alt_ft.cmp(&a.alt_ft));
                        }
                        SortKind::SpeedDesc => {
                            flights.sort_by(|a, b| b.speed_kts.total_cmp(&a.speed_kts));
                        }
                        SortKind::CallsignAsc => {
                            flights.sort_by(|a, b| a.callsign.cmp(&b.callsign));
                        }
                    }

                    // Table Header Bar with Sort Selector
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("ACTIVE TRACKS ({})", flights.len()))
                                .size(10.5)
                                .monospace()
                                .strong()
                                .color(Color32::from_rgb(148, 163, 184)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            egui::ComboBox::from_id_salt("sort_tracks_combo")
                                .width(110.0)
                                .selected_text(match self.sort_by {
                                    SortKind::KurdistanFirst => "Sort: Priority",
                                    SortKind::AltitudeDesc => "Sort: Altitude",
                                    SortKind::SpeedDesc => "Sort: Speed",
                                    SortKind::CallsignAsc => "Sort: Callsign",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.sort_by,
                                        SortKind::KurdistanFirst,
                                        "Priority (KRI / Mil)",
                                    );
                                    ui.selectable_value(
                                        &mut self.sort_by,
                                        SortKind::AltitudeDesc,
                                        "Altitude (High → Low)",
                                    );
                                    ui.selectable_value(
                                        &mut self.sort_by,
                                        SortKind::SpeedDesc,
                                        "Speed (Fast → Slow)",
                                    );
                                    ui.selectable_value(
                                        &mut self.sort_by,
                                        SortKind::CallsignAsc,
                                        "Callsign (A → Z)",
                                    );
                                });
                        });
                    });

                    ui.separator();

                    // High-Density Tactical Track List
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = vec2(0.0, 3.0);
                            for f in flights {
                                let is_selected = self.selected_hex.as_deref() == Some(&f.hex);
                                let track_col = flight_color(f, false);

                                let row_bg = if is_selected {
                                    Color32::from_rgb(28, 42, 68)
                                } else if f.is_military {
                                    Color32::from_rgba_unmultiplied(45, 24, 16, 180)
                                } else if f.in_kri {
                                    Color32::from_rgba_unmultiplied(32, 28, 16, 150)
                                } else {
                                    Color32::from_rgb(13, 19, 30)
                                };

                                let stroke_col = if is_selected {
                                    Color32::from_rgb(251, 191, 36)
                                } else if f.is_military {
                                    Color32::from_rgba_unmultiplied(249, 115, 22, 120)
                                } else {
                                    Color32::from_rgb(24, 34, 52)
                                };

                                let frame_resp = egui::Frame::NONE
                                    .fill(row_bg)
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(egui::Margin::symmetric(8, 5))
                                    .stroke(Stroke::new(1.0, stroke_col))
                                    .show(ui, |ui| {
                                        // Line 1: Callsign + Badge + Model + Altitude
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&f.callsign)
                                                    .monospace()
                                                    .strong()
                                                    .size(12.5)
                                                    .color(Color32::WHITE),
                                            );
                                            if !f.model_code.is_empty() {
                                                ui.label(
                                                    egui::RichText::new(&f.model_code)
                                                        .monospace()
                                                        .size(10.0)
                                                        .color(Color32::from_rgb(148, 163, 184)),
                                                );
                                            }
                                            if f.is_military {
                                                ui.label(
                                                    egui::RichText::new("MIL")
                                                        .monospace()
                                                        .strong()
                                                        .size(9.5)
                                                        .color(Color32::from_rgb(249, 115, 22)),
                                                );
                                            } else if f.in_kri {
                                                ui.label(
                                                    egui::RichText::new("KRI")
                                                        .monospace()
                                                        .strong()
                                                        .size(9.5)
                                                        .color(Color32::from_rgb(251, 191, 36)),
                                                );
                                            }

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    let alt_txt = if f.on_ground {
                                                        "GND".to_string()
                                                    } else {
                                                        format!("FL{:03}", (f.alt_ft / 100).max(0))
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(alt_txt)
                                                            .color(track_col)
                                                            .monospace()
                                                            .strong()
                                                            .size(11.0),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "{:.0}KT",
                                                            f.speed_kts
                                                        ))
                                                        .color(Color32::from_rgb(148, 163, 184))
                                                        .monospace()
                                                        .size(10.5),
                                                    );
                                                },
                                            );
                                        });

                                        // Line 2: Route + Kurdish Region
                                        ui.horizontal(|ui| {
                                            let route_str =
                                                if !f.origin.is_empty() || !f.dest.is_empty() {
                                                    let o = if f.origin.is_empty() {
                                                        "---"
                                                    } else {
                                                        &f.origin
                                                    };
                                                    let d = if f.dest.is_empty() {
                                                        "---"
                                                    } else {
                                                        &f.dest
                                                    };
                                                    format!("{o} → {d}")
                                                } else {
                                                    f.airline.clone()
                                                };
                                            ui.label(
                                                egui::RichText::new(route_str)
                                                    .size(10.5)
                                                    .color(Color32::from_rgb(148, 163, 184)),
                                            );

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if let Some(ref reg) = f.region {
                                                        let short_reg =
                                                            reg.split(" (").next().unwrap_or(reg);
                                                        ui.label(
                                                            egui::RichText::new(short_reg)
                                                                .size(10.0)
                                                                .color(if f.in_kri {
                                                                    Color32::from_rgb(251, 191, 36)
                                                                } else {
                                                                    Color32::from_rgb(56, 189, 248)
                                                                }),
                                                        );
                                                    } else {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "{:.0}NM EBL",
                                                                f.dist_erbil_nm
                                                            ))
                                                            .size(10.0)
                                                            .monospace()
                                                            .color(Color32::from_rgb(100, 116, 139)),
                                                        );
                                                    }
                                                },
                                            );
                                        });
                                    });

                                let click_resp = ui.interact(
                                    frame_resp.response.rect,
                                    ui.id().with(&f.hex),
                                    Sense::click(),
                                );
                                if click_resp.clicked() {
                                    self.selected_hex = Some(f.hex.clone());
                                    if let Some(tr) = self.tracks.get(&f.hex) {
                                        self.map_memory.center_at(lat_lon(
                                            tr.display_lat,
                                            tr.display_lon,
                                        ));
                                    } else {
                                        self.map_memory.center_at(lat_lon(f.lat, f.lon));
                                    }
                                }
                            }
                        });
                });
        }

        // ── 3. CENTRAL TACTICAL MAP CANVAS & FLOATING HUD CONTROLS ──────────────
        egui::CentralPanel::default().show(ui, |ui| {
            let map_rect = ui.max_rect();
            let current_zoom = self.map_memory.zoom();

            let overlay = OverlayPlugin {
                atlas: &self.atlas,
                show_borders: self.show_borders,
                show_disputed: self.show_disputed,
                show_provinces: self.show_provinces,
                show_luristan: self.show_luristan,
                show_places: self.show_places,
                show_airports: self.show_airports,
                show_trails: self.show_trails,
                show_vectors: self.show_vectors,
                flights: self
                    .snapshot
                    .as_ref()
                    .map(|s| s.flights.as_slice())
                    .unwrap_or(&[]),
                tracks: &self.tracks,
                selected_hex: self.selected_hex.as_deref(),
                zoom: current_zoom,
            };

            let map_widget = Map::new(
                Some(&mut self.tiles),
                &mut self.map_memory,
                lat_lon(KURDISTAN_CENTER.0, KURDISTAN_CENTER.1),
            )
            .with_plugin(overlay);

            ui.add(map_widget);

            // Floating Map Zoom & Reset HUD (Bottom-Right)
            let zoom_rect = Rect::from_min_size(
                Pos2::new(map_rect.right() - 48.0, map_rect.bottom() - 132.0),
                vec2(36.0, 112.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(zoom_rect), |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(11, 16, 26, 235))
                    .corner_radius(CornerRadius::same(6))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(38, 52, 74)))
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            if ui
                                .add_sized([28.0, 26.0], egui::Button::new(egui::RichText::new("+").size(16.0).strong()))
                                .on_hover_text("Zoom In")
                                .clicked()
                            {
                                self.map_memory.zoom_in().ok();
                            }
                            if ui
                                .add_sized([28.0, 26.0], egui::Button::new(egui::RichText::new("−").size(16.0).strong()))
                                .on_hover_text("Zoom Out")
                                .clicked()
                            {
                                self.map_memory.zoom_out().ok();
                            }
                            if ui
                                .add_sized([28.0, 26.0], egui::Button::new(egui::RichText::new("⌂").size(14.0)))
                                .on_hover_text("Reset Kurdistan Overview")
                                .clicked()
                            {
                                self.jump_to_sector(KURDISTAN_CENTER.0, KURDISTAN_CENTER.1, 6.6);
                            }
                        });
                    });
            });

            // Floating Legend Bar (Bottom-Left of Map)
            let legend_rect = Rect::from_min_size(
                Pos2::new(map_rect.left() + 12.0, map_rect.bottom() - 36.0),
                vec2(540.0, 26.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(legend_rect), |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(9, 13, 21, 220))
                    .corner_radius(CornerRadius::same(4))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(30, 41, 59)))
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(10.0, 0.0);
                            legend_dot(ui, Color32::from_rgb(251, 191, 36), "Herêma Kurdistanê (KRI)");
                            legend_dot(ui, Color32::from_rgb(56, 189, 248), "Kurdistana Mezin");
                            legend_dot(ui, Color32::from_rgb(245, 158, 11), "Disputed (Art.140)");
                            ui.separator();
                            legend_dot(ui, Color32::from_rgb(16, 185, 129), "<10k ft");
                            legend_dot(ui, Color32::from_rgb(59, 130, 246), "Cruise");
                            legend_dot(ui, Color32::from_rgb(168, 85, 247), ">36k ft");
                            legend_dot(ui, Color32::from_rgb(249, 115, 22), "Mil/Gov");
                        });
                    });
            });
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

impl RadarApp {
    fn render_target_dossier(&mut self, ui: &mut egui::Ui, f: &Flight) {
        egui::Frame::NONE
            .fill(Color32::from_rgb(15, 23, 38))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::same(10))
            .stroke(Stroke::new(1.2, Color32::from_rgb(251, 191, 36)))
            .show(ui, |ui| {
                // Header: Callsign + Category Badge + Close
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&f.callsign)
                            .monospace()
                            .strong()
                            .size(16.0)
                            .color(Color32::WHITE),
                    );
                    if f.is_military {
                        ui.label(
                            egui::RichText::new(" [MIL / GOV] ")
                                .monospace()
                                .strong()
                                .size(10.5)
                                .color(Color32::from_rgb(249, 115, 22)),
                        );
                    } else if f.in_kri {
                        ui.label(
                            egui::RichText::new(" [KRI AIRSPACE] ")
                                .monospace()
                                .strong()
                                .size(10.5)
                                .color(Color32::from_rgb(251, 191, 36)),
                        );
                    } else if f.in_greater {
                        ui.label(
                            egui::RichText::new(" [KURDISTAN] ")
                                .monospace()
                                .strong()
                                .size(10.5)
                                .color(Color32::from_rgb(56, 189, 248)),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").on_hover_text("Close Dossier (Esc)").clicked() {
                            self.selected_hex = None;
                            self.follow_hex = None;
                        }
                        let following = self.follow_hex.as_deref() == Some(&f.hex);
                        let lock_txt = if following { "LOCKED" } else { "TRACK" };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(lock_txt)
                                        .monospace()
                                        .size(10.0)
                                        .strong()
                                        .color(if following {
                                            Color32::BLACK
                                        } else {
                                            Color32::from_rgb(251, 191, 36)
                                        }),
                                )
                                .fill(if following {
                                    Color32::from_rgb(251, 191, 36)
                                } else {
                                    Color32::from_rgb(24, 35, 56)
                                }),
                            )
                            .clicked()
                        {
                            if following {
                                self.follow_hex = None;
                            } else {
                                self.follow_hex = Some(f.hex.clone());
                            }
                        }
                    });
                });

                // Airline & Aircraft Type
                ui.label(
                    egui::RichText::new(format!("{} · {}", f.airline, f.model_name))
                        .size(11.5)
                        .color(Color32::from_rgb(203, 213, 225)),
                );

                // Route Bar
                if !f.origin.is_empty() || !f.dest.is_empty() {
                    ui.add_space(4.0);
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(10, 15, 25))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let orig = if f.origin.is_empty() { "---" } else { &f.origin };
                                let dest = if f.dest.is_empty() { "---" } else { &f.dest };
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{orig} ({})  →  {dest} ({})",
                                        f.origin_city, f.dest_city
                                    ))
                                    .monospace()
                                    .strong()
                                    .size(11.0)
                                    .color(Color32::from_rgb(251, 191, 36)),
                                );
                            });
                        });
                }

                ui.add_space(4.0);

                // Sector / Province Banner
                let sector_str = match (&f.region, &f.part) {
                    (Some(r), Some(p)) => format!("SECTOR: {r} · {p}"),
                    (Some(r), None) => format!("SECTOR: {r}"),
                    _ => "SECTOR: INTERNATIONAL / ADJACENT".to_string(),
                };
                ui.label(
                    egui::RichText::new(sector_str)
                        .monospace()
                        .size(10.5)
                        .strong()
                        .color(Color32::from_rgb(56, 189, 248)),
                );

                ui.add_space(4.0);

                // Compact Telemetry Grid
                egui::Grid::new("c2_dossier_grid")
                    .num_columns(4)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        dossier_kv(
                            ui,
                            "ALT",
                            &format!("{} ft", f.alt_ft),
                            Color32::WHITE,
                        );
                        dossier_kv(
                            ui,
                            "GS",
                            &format!("{:.0} kt", f.speed_kts),
                            Color32::WHITE,
                        );
                        ui.end_row();

                        dossier_kv(
                            ui,
                            "TRK",
                            &format!("{:03.0}° {}", f.heading, cardinal(f.heading as f64)),
                            Color32::WHITE,
                        );
                        dossier_kv(
                            ui,
                            "VS",
                            &format!("{:+} fpm", f.vrate_fpm),
                            if f.vrate_fpm > 300 {
                                Color32::from_rgb(16, 185, 129)
                            } else if f.vrate_fpm < -300 {
                                Color32::from_rgb(245, 158, 11)
                            } else {
                                Color32::from_rgb(148, 163, 184)
                            },
                        );
                        ui.end_row();

                        dossier_kv(
                            ui,
                            "HEX",
                            &f.hex,
                            Color32::from_rgb(148, 163, 184),
                        );
                        dossier_kv(
                            ui,
                            "REG",
                            if f.registration.is_empty() {
                                "N/A"
                            } else {
                                &f.registration
                            },
                            Color32::from_rgb(148, 163, 184),
                        );
                        ui.end_row();

                        dossier_kv(
                            ui,
                            "SQK",
                            if f.squawk.is_empty() { "----" } else { &f.squawk },
                            if f.is_emergency() {
                                Color32::from_rgb(239, 68, 68)
                            } else {
                                Color32::from_rgb(251, 191, 36)
                            },
                        );
                        dossier_kv(
                            ui,
                            "SRC",
                            &f.source,
                            Color32::from_rgb(16, 185, 129),
                        );
                        ui.end_row();

                        dossier_kv(
                            ui,
                            "ORER",
                            &format!("{:.0}NM {:03.0}°", f.dist_erbil_nm, f.radial_erbil),
                            Color32::from_rgb(203, 213, 225),
                        );
                        dossier_kv(
                            ui,
                            "ORSU",
                            &format!("{:.0}NM {:03.0}°", f.dist_suly_nm, f.radial_suly),
                            Color32::from_rgb(203, 213, 225),
                        );
                        ui.end_row();
                    });
            });
    }
}

fn tactical_stat_pill(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(15, 22, 34))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(7, 3))
        .stroke(Stroke::new(1.0, Color32::from_rgb(30, 41, 59)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(5.0, 0.0);
                ui.label(
                    egui::RichText::new(label)
                        .size(10.0)
                        .monospace()
                        .color(Color32::from_rgb(148, 163, 184)),
                );
                ui.label(
                    egui::RichText::new(value)
                        .size(11.5)
                        .monospace()
                        .strong()
                        .color(color),
                );
            });
        });
}

fn sector_btn(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .size(10.0)
                .monospace()
                .strong()
                .color(Color32::from_rgb(203, 213, 225)),
        )
        .fill(Color32::from_rgb(18, 26, 40)),
    )
}

fn layer_chip(ui: &mut egui::Ui, active: &mut bool, label: &str) {
    let bg = if *active {
        Color32::from_rgb(23, 43, 69)
    } else {
        Color32::from_rgb(15, 21, 32)
    };
    let fg = if *active {
        Color32::from_rgb(56, 189, 248)
    } else {
        Color32::from_rgb(100, 116, 139)
    };
    if ui
        .add(
            egui::Button::new(egui::RichText::new(label).size(10.5).color(fg))
                .fill(bg)
                .stroke(Stroke::new(
                    1.0,
                    if *active {
                        Color32::from_rgba_unmultiplied(56, 189, 248, 120)
                    } else {
                        Color32::from_rgb(30, 41, 59)
                    },
                )),
        )
        .clicked()
    {
        *active = !*active;
    }
}

fn dossier_kv(ui: &mut egui::Ui, k: &str, v: &str, v_col: Color32) {
    ui.label(
        egui::RichText::new(k)
            .size(10.0)
            .monospace()
            .color(Color32::from_rgb(100, 116, 139)),
    );
    ui.label(
        egui::RichText::new(v)
            .size(11.0)
            .monospace()
            .strong()
            .color(v_col),
    );
}

fn legend_dot(ui: &mut egui::Ui, col: Color32, text: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
        let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 3.5, col);
        ui.label(
            egui::RichText::new(text)
                .size(10.0)
                .color(Color32::from_rgb(148, 163, 184)),
        );
    });
}

fn flight_color(f: &Flight, is_selected: bool) -> Color32 {
    if is_selected {
        Color32::from_rgb(251, 191, 36)
    } else if f.is_emergency() {
        Color32::from_rgb(239, 68, 68)
    } else if f.is_military {
        Color32::from_rgb(249, 115, 22)
    } else if f.on_ground || f.alt_ft <= 500 {
        Color32::from_rgb(148, 163, 184)
    } else if f.alt_ft < 12000 {
        Color32::from_rgb(16, 185, 129)
    } else if f.alt_ft < 26000 {
        Color32::from_rgb(6, 182, 212)
    } else if f.alt_ft < 36000 {
        Color32::from_rgb(59, 130, 246)
    } else {
        Color32::from_rgb(168, 85, 247)
    }
}

/// Custom GPU map rendering plugin for Kurdistan borders, disputed territories,
/// curated Kurdish cities/peaks, airport beacons, and dead-reckoned aircraft.
struct OverlayPlugin<'a> {
    atlas: &'a Atlas,
    show_borders: bool,
    show_disputed: bool,
    show_provinces: bool,
    show_luristan: bool,
    show_places: bool,
    show_airports: bool,
    show_trails: bool,
    show_vectors: bool,
    flights: &'a [Flight],
    tracks: &'a HashMap<String, VisualTrack>,
    selected_hex: Option<&'a str>,
    zoom: f64,
}

impl<'a> Plugin for OverlayPlugin<'a> {
    fn run(
        self: Box<Self>,
        ui: &mut egui::Ui,
        response: &egui::Response,
        projector: &Projector,
        _map_memory: &MapMemory,
    ) {
        let painter = ui.painter();
        let clip_rect = ui.clip_rect().expand(120.0);
        let hover_pos = response.hover_pos();
        let clicked_pos = if response.clicked() {
            response.interact_pointer_pos()
        } else {
            None
        };

        // ── 1. KURDISTAN ATLAS BOUNDARIES ───────────────────────────────────────
        if self.show_borders {
            // 1a. Major Kurdish Lakes (Gola Wanê, Gola Urmiyê, Dûkan, Derbendîxan)
            for lake in self.atlas.lakes() {
                if !bbox_on_screen(&lake.bbox, projector, &clip_rect) {
                    continue;
                }
                for ring in &lake.rings {
                    let pts = project_ring(ring, projector);
                    if pts.len() >= 3 {
                        painter.add(egui::Shape::line(
                            pts,
                            Stroke::new(1.1, Color32::from_rgba_unmultiplied(14, 165, 233, 85)),
                        ));
                    }
                }
            }

            // 1b. Optional Internal ADM1 Province Grid
            if self.show_provinces {
                for reg in self.atlas.greater() {
                    if !bbox_on_screen(&reg.bbox, projector, &clip_rect) {
                        continue;
                    }
                    for ring in &reg.rings {
                        let pts = project_ring(ring, projector);
                        if pts.len() >= 2 {
                            painter.add(egui::Shape::line(
                                pts,
                                Stroke::new(
                                    0.9,
                                    Color32::from_rgba_unmultiplied(56, 189, 248, 42),
                                ),
                            ));
                        }
                    }
                }
            }

            // 1c. Internal 4-Part Division Lines (Bakur / Başûr / Rojava / Rojhilat)
            for seg in &self.atlas.part_borders {
                let pts = project_ring(seg, projector);
                draw_dashed_polyline(
                    painter,
                    &pts,
                    7.0,
                    5.0,
                    Stroke::new(1.3, Color32::from_rgba_unmultiplied(148, 163, 184, 125)),
                );
            }

            // 1d. Optional Luristan Southern Zagros Extent
            if self.show_luristan {
                if let Some(lur) = self.atlas.luristan_outline() {
                    if bbox_on_screen(&lur.bbox, projector, &clip_rect) {
                        for ring in &lur.rings {
                            let pts = project_ring(ring, projector);
                            draw_dashed_polyline(
                                painter,
                                &pts,
                                8.0,
                                4.0,
                                Stroke::new(
                                    1.6,
                                    Color32::from_rgba_unmultiplied(45, 212, 191, 175),
                                ),
                            );
                        }
                    }
                }
            }

            // 1e. Disputed Territories (Article 140 & Rojava Contested Districts)
            if self.show_disputed {
                for reg in self.atlas.disputed() {
                    if !bbox_on_screen(&reg.bbox, projector, &clip_rect) {
                        continue;
                    }
                    let stroke_col = if reg.control.as_deref() == Some("krg") {
                        Color32::from_rgba_unmultiplied(251, 191, 36, 130)
                    } else {
                        Color32::from_rgba_unmultiplied(245, 158, 11, 145)
                    };
                    for ring in &reg.rings {
                        let pts = project_ring(ring, projector);
                        draw_dashed_polyline(
                            painter,
                            &pts,
                            6.0,
                            4.0,
                            Stroke::new(1.25, stroke_col),
                        );
                    }
                }
            }

            // 1f. Primary Continuous Outer Perimeter of Greater Kurdistan (`Kurdistana Mezin`)
            if let Some(outline) = self.atlas.primary_outline() {
                if bbox_on_screen(&outline.bbox, projector, &clip_rect) {
                    for ring in &outline.rings {
                        let pts = project_ring(ring, projector);
                        if pts.len() >= 2 {
                            // Soft outer tactical glow
                            painter.add(egui::Shape::line(
                                pts.clone(),
                                Stroke::new(
                                    5.0,
                                    Color32::from_rgba_unmultiplied(56, 189, 248, 36),
                                ),
                            ));
                            // Crisp perimeter line
                            painter.add(egui::Shape::line(
                                pts,
                                Stroke::new(
                                    2.1,
                                    Color32::from_rgba_unmultiplied(56, 189, 248, 215),
                                ),
                            ));
                        }
                    }
                }
            }

            // 1g. Kurdistan Region of Iraq (`Herêma Kurdistanê` — KRI)
            for reg in self.atlas.kri() {
                if !bbox_on_screen(&reg.bbox, projector, &clip_rect) {
                    continue;
                }
                for ring in &reg.rings {
                    let pts = project_ring(ring, projector);
                    if pts.len() >= 2 {
                        painter.add(egui::Shape::line(
                            pts.clone(),
                            Stroke::new(
                                4.5,
                                Color32::from_rgba_unmultiplied(251, 191, 36, 40),
                            ),
                        ));
                        painter.add(egui::Shape::line(
                            pts,
                            Stroke::new(
                                2.2,
                                Color32::from_rgba_unmultiplied(251, 191, 36, 235),
                            ),
                        ));
                    }
                }
            }
        }

        // ── 2. CURATED KURDISH PLACES, CAPITALS, REGIONS & PEAKS ────────────────
        let mut occupied_labels: Vec<Rect> = Vec::with_capacity(96);

        if self.show_places {
            for place in &self.atlas.places {
                if place.region == "luristan" && !self.show_luristan && place.id != "xurremabad" {
                    continue;
                }

                // Zoom-based Level of Detail (LOD)
                let visible_at_zoom = match place.category {
                    PlaceCategory::RegionLabel => self.zoom >= 5.0 && self.zoom <= 8.6,
                    PlaceCategory::Capital | PlaceCategory::HistoricalCapital => self.zoom >= 5.2,
                    PlaceCategory::City => {
                        if place.priority <= 1 {
                            self.zoom >= 6.0
                        } else {
                            self.zoom >= 7.3
                        }
                    }
                    PlaceCategory::Peak => self.zoom >= 7.1,
                };
                if !visible_at_zoom {
                    continue;
                }

                let pos = projector.project(Position::new(place.lon, place.lat)).to_pos2();
                if !clip_rect.contains(pos) {
                    continue;
                }

                match place.category {
                    PlaceCategory::RegionLabel => {
                        let text = place.name_ku.to_uppercase();
                        let label_rect = Rect::from_center_size(pos, vec2(90.0, 20.0));
                        if !overlaps(&occupied_labels, label_rect) {
                            occupied_labels.push(label_rect);
                            draw_halo_text(
                                painter,
                                pos,
                                &text,
                                FontId::proportional(14.0),
                                Color32::from_rgba_unmultiplied(226, 232, 240, 145),
                            );
                        }
                    }
                    PlaceCategory::Capital | PlaceCategory::HistoricalCapital => {
                        let label_pos = pos + vec2(0.0, -10.0);
                        let est_w = (place.name_ku.len() as f32 * 7.2).max(46.0);
                        let label_rect = Rect::from_center_size(label_pos, vec2(est_w, 16.0));
                        if !overlaps(&occupied_labels, label_rect) {
                            occupied_labels.push(label_rect);
                            let col = if place.category == PlaceCategory::Capital {
                                Color32::from_rgb(251, 191, 36)
                            } else {
                                Color32::from_rgb(56, 189, 248)
                            };
                            draw_diamond(painter, pos, 4.2, col);
                            draw_halo_text(
                                painter,
                                label_pos,
                                &place.name_ku,
                                FontId::proportional(12.0),
                                Color32::from_rgb(248, 250, 252),
                            );
                        }
                    }
                    PlaceCategory::City => {
                        let label_pos = pos + vec2(0.0, -8.0);
                        let est_w = (place.name_ku.len() as f32 * 6.5).max(40.0);
                        let label_rect = Rect::from_center_size(label_pos, vec2(est_w, 14.0));
                        if !overlaps(&occupied_labels, label_rect) {
                            occupied_labels.push(label_rect);
                            painter.circle_filled(
                                pos,
                                if place.priority <= 1 { 2.6 } else { 2.0 },
                                Color32::from_rgba_unmultiplied(203, 213, 225, 210),
                            );
                            painter.circle_stroke(
                                pos,
                                3.2,
                                Stroke::new(1.0, Color32::from_rgb(7, 10, 16)),
                            );
                            draw_halo_text(
                                painter,
                                label_pos,
                                &place.name_ku,
                                FontId::proportional(if place.priority <= 1 { 11.0 } else { 10.0 }),
                                Color32::from_rgba_unmultiplied(226, 232, 240, 225),
                            );
                        }
                    }
                    PlaceCategory::Peak => {
                        let label_text = match place.elevation_m {
                            Some(m) => format!("{} ({m}m)", place.name_ku),
                            None => place.name_ku.clone(),
                        };
                        let label_pos = pos + vec2(0.0, 9.0);
                        let est_w = (label_text.len() as f32 * 5.8).max(48.0);
                        let label_rect = Rect::from_center_size(label_pos, vec2(est_w, 13.0));
                        if !overlaps(&occupied_labels, label_rect) {
                            occupied_labels.push(label_rect);
                            draw_peak_triangle(
                                painter,
                                pos,
                                4.2,
                                Color32::from_rgba_unmultiplied(167, 243, 208, 210),
                            );
                            draw_halo_text(
                                painter,
                                label_pos,
                                &label_text,
                                FontId::proportional(9.5),
                                Color32::from_rgba_unmultiplied(167, 243, 208, 210),
                            );
                        }
                    }
                }
            }
        }

        // ── 3. AIRPORT BEACONS ──────────────────────────────────────────────────
        let mut hovered_airport = None;
        if self.show_airports {
            for apt in AIRPORTS {
                if !apt.kurdish && self.zoom < 6.2 {
                    continue;
                }
                let pos = projector.project(lat_lon(apt.lat, apt.lon)).to_pos2();
                if !clip_rect.contains(pos) {
                    continue;
                }

                let col = if apt.kri_hub {
                    Color32::from_rgb(251, 191, 36)
                } else if apt.kurdish {
                    Color32::from_rgb(56, 189, 248)
                } else {
                    Color32::from_rgba_unmultiplied(148, 163, 184, 150)
                };

                painter.circle_filled(pos, if apt.kri_hub { 3.8 } else { 2.8 }, col);
                if apt.kurdish {
                    painter.circle_stroke(pos, if apt.kri_hub { 7.5 } else { 5.5 }, Stroke::new(1.1, col));
                    if self.zoom >= 6.0 || apt.kri_hub {
                        let tag_pos = pos + vec2(0.0, 11.0);
                        draw_halo_text(
                            painter,
                            tag_pos,
                            apt.iata,
                            FontId::monospace(9.5),
                            col,
                        );
                    }
                }

                if let Some(hp) = hover_pos {
                    if (hp - pos).length() < 12.0 {
                        hovered_airport = Some((apt, pos));
                    }
                }
            }
        }

        // ── 4. AIRCRAFT TRACKS, TRAILS, VELOCITY VECTORS & SILHOUETTES ──────────
        let mut clicked_hex: Option<String> = None;
        let mut hovered_flight: Option<(&Flight, Pos2)> = None;

        for f in self.flights {
            let is_selected = self.selected_hex == Some(f.hex.as_str());
            let (lat, lon, heading) = match self.tracks.get(&f.hex) {
                Some(tr) => (tr.display_lat, tr.display_lon, tr.display_heading),
                None => (f.lat, f.lon, f.heading),
            };

            let screen_pos = projector.project(lat_lon(lat, lon)).to_pos2();
            if !clip_rect.contains(screen_pos) && !is_selected {
                continue;
            }

            let col = flight_color(f, is_selected);

            // Hit test for click and hover
            if let Some(mp) = clicked_pos {
                if (screen_pos - mp).length() < 16.0 {
                    clicked_hex = Some(f.hex.clone());
                }
            }
            if let Some(hp) = hover_pos {
                if (screen_pos - hp).length() < 15.0 {
                    hovered_flight = Some((f, screen_pos));
                }
            }

            // Historical Trail
            if (self.show_trails || is_selected) && f.trail.len() >= 2 {
                let mut trail_pts: Vec<Pos2> = f
                    .trail
                    .iter()
                    .map(|pt| projector.project(lat_lon(pt[0] as f64, pt[1] as f64)).to_pos2())
                    .collect();
                trail_pts.push(screen_pos);

                painter.add(egui::Shape::line(
                    trail_pts,
                    Stroke::new(
                        if is_selected { 2.2 } else { 1.3 },
                        Color32::from_rgba_unmultiplied(
                            col.r(),
                            col.g(),
                            col.b(),
                            if is_selected { 200 } else { 105 },
                        ),
                    ),
                ));
            }

            // 60-Second Projected Velocity Vector Line
            if (self.show_vectors || is_selected) && !f.on_ground && f.speed_kts > 40.0 {
                let (v_lat, v_lon) =
                    dead_reckon(lat, lon, heading as f64, f.speed_kts as f64, 60.0);
                let vec_end = projector.project(lat_lon(v_lat, v_lon)).to_pos2();
                painter.line_segment(
                    [screen_pos, vec_end],
                    Stroke::new(
                        1.1,
                        Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 155),
                    ),
                );
                painter.circle_filled(
                    vec_end,
                    1.8,
                    Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 180),
                );
            }

            // Swept-Wing Aircraft Silhouette
            let scale = if is_selected {
                12.5
            } else if f.widebody || f.is_military {
                10.5
            } else {
                8.8
            };
            draw_aircraft_icon(painter, screen_pos, heading.to_radians(), scale, col);

            // Target Lock Reticle for Selected or Military Aircraft
            if is_selected {
                draw_target_reticle(painter, screen_pos, 16.0, Color32::from_rgb(251, 191, 36));
            } else if f.is_military {
                draw_diamond_outline(
                    painter,
                    screen_pos,
                    12.0,
                    Color32::from_rgba_unmultiplied(249, 115, 22, 160),
                );
            }

            // Callsign Tag on Map
            if is_selected
                || f.is_military
                || (f.in_kri && self.zoom >= 6.4)
                || (f.in_greater && self.zoom >= 7.2)
                || self.zoom >= 8.0
            {
                let tag_text = if is_selected {
                    format!(
                        "{} · FL{:03} · {:.0}KT",
                        f.callsign,
                        (f.alt_ft / 100).max(0),
                        f.speed_kts
                    )
                } else {
                    f.callsign.clone()
                };
                let tag_pos = screen_pos + vec2(0.0, -14.0);
                draw_halo_text(
                    painter,
                    tag_pos,
                    &tag_text,
                    FontId::monospace(if is_selected { 10.5 } else { 9.5 }),
                    if is_selected {
                        Color32::from_rgb(251, 191, 36)
                    } else {
                        Color32::WHITE
                    },
                );
            }
        }

        // ── 5. HOVER TOOLTIPS (AIRCRAFT OR AIRPORT) ─────────────────────────────
        if let Some((f, pos)) = hovered_flight {
            let tip_lines = [
                format!("{} ({})", f.callsign, f.model_name),
                format!(
                    "ALT: {} ft  ·  GS: {:.0} kt  ·  HDG: {:03.0}°",
                    f.alt_ft, f.speed_kts, f.heading
                ),
                if !f.origin.is_empty() || !f.dest.is_empty() {
                    format!("ROUTE: {} → {}", f.origin_city, f.dest_city)
                } else {
                    format!("OPR: {}", f.airline)
                },
                f.region
                    .as_ref()
                    .map(|r| format!("SECTOR: {r}"))
                    .unwrap_or_else(|| format!("EBL: {:.0} NM", f.dist_erbil_nm)),
            ];
            draw_tactical_tooltip(painter, pos + vec2(16.0, -16.0), &tip_lines);
        } else if let Some((apt, pos)) = hovered_airport {
            let tip_lines = [
                format!("{} / {} — {}", apt.iata, apt.icao, apt.city),
                apt.name.to_string(),
                format!("Sector: {}", apt.country),
            ];
            draw_tactical_tooltip(painter, pos + vec2(14.0, -14.0), &tip_lines);
        }

        if let Some(hex) = clicked_hex {
            ui.data_mut(|d| {
                d.insert_temp(egui::Id::new("clicked_flight_hex"), hex);
            });
        }
    }
}

fn bbox_on_screen(bbox: &[f64; 4], projector: &Projector, clip_rect: &Rect) -> bool {
    let p_min = projector.project(Position::new(bbox[0], bbox[3])).to_pos2();
    let p_max = projector.project(Position::new(bbox[2], bbox[1])).to_pos2();
    let poly_rect = Rect::from_two_pos(p_min, p_max);
    clip_rect.intersects(poly_rect)
}

fn project_ring(ring: &[[f64; 2]], projector: &Projector) -> Vec<Pos2> {
    ring.iter()
        .map(|p| projector.project(Position::new(p[0], p[1])).to_pos2())
        .collect()
}

fn overlaps(occupied: &[Rect], candidate: Rect) -> bool {
    occupied.iter().any(|r| r.intersects(candidate))
}

fn draw_halo_text(painter: &egui::Painter, pos: Pos2, text: &str, font: FontId, color: Color32) {
    let halo = Color32::from_rgba_unmultiplied(5, 8, 14, 230);
    for offset in [
        vec2(-1.0, -1.0),
        vec2(1.0, -1.0),
        vec2(-1.0, 1.0),
        vec2(1.0, 1.0),
        vec2(0.0, 1.5),
    ] {
        painter.text(
            pos + offset,
            Align2::CENTER_CENTER,
            text,
            font.clone(),
            halo,
        );
    }
    painter.text(pos, Align2::CENTER_CENTER, text, font, color);
}

fn draw_diamond(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let pts = vec![
        center + vec2(0.0, -r),
        center + vec2(r, 0.0),
        center + vec2(0.0, r),
        center + vec2(-r, 0.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        pts,
        color,
        Stroke::new(1.0, Color32::from_rgb(6, 9, 15)),
    ));
}

fn draw_diamond_outline(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let pts = vec![
        center + vec2(0.0, -r),
        center + vec2(r, 0.0),
        center + vec2(0.0, r),
        center + vec2(-r, 0.0),
        center + vec2(0.0, -r),
    ];
    painter.add(egui::Shape::line(pts, Stroke::new(1.2, color)));
}

fn draw_peak_triangle(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let pts = vec![
        center + vec2(0.0, -r),
        center + vec2(r * 0.95, r * 0.8),
        center + vec2(-r * 0.95, r * 0.8),
    ];
    painter.add(egui::Shape::convex_polygon(
        pts,
        color,
        Stroke::new(1.0, Color32::from_rgb(6, 9, 15)),
    ));
}

fn draw_target_reticle(painter: &egui::Painter, c: Pos2, r: f32, col: Color32) {
    let s = r * 0.42;
    let stroke = Stroke::new(1.6, col);
    // Top-left
    painter.line_segment([c + vec2(-r, -r), c + vec2(-r + s, -r)], stroke);
    painter.line_segment([c + vec2(-r, -r), c + vec2(-r, -r + s)], stroke);
    // Top-right
    painter.line_segment([c + vec2(r, -r), c + vec2(r - s, -r)], stroke);
    painter.line_segment([c + vec2(r, -r), c + vec2(r, -r + s)], stroke);
    // Bottom-left
    painter.line_segment([c + vec2(-r, r), c + vec2(-r + s, r)], stroke);
    painter.line_segment([c + vec2(-r, r), c + vec2(-r, r - s)], stroke);
    // Bottom-right
    painter.line_segment([c + vec2(r, r), c + vec2(r - s, r)], stroke);
    painter.line_segment([c + vec2(r, r), c + vec2(r, r - s)], stroke);
}

/// Renders a realistic swept-wing aircraft silhouette using two convex polygons
/// (fuselage + swept wings and tailplane) rotated to true heading.
fn draw_aircraft_icon(
    painter: &egui::Painter,
    pos: Pos2,
    heading_rad: f32,
    s: f32,
    color: Color32,
) {
    let outline = Stroke::new(1.0, Color32::from_rgb(6, 9, 15));

    // Main swept wing
    let wings = vec![
        pos + rotate_vec(vec2(0.0, -s * 0.35), heading_rad),
        pos + rotate_vec(vec2(s * 0.95, s * 0.22), heading_rad),
        pos + rotate_vec(vec2(s * 0.85, s * 0.38), heading_rad),
        pos + rotate_vec(vec2(0.0, s * 0.08), heading_rad),
        pos + rotate_vec(vec2(-s * 0.85, s * 0.38), heading_rad),
        pos + rotate_vec(vec2(-s * 0.95, s * 0.22), heading_rad),
    ];
    painter.add(egui::Shape::convex_polygon(wings, color, outline));

    // Tailplane
    let tail = vec![
        pos + rotate_vec(vec2(0.0, s * 0.52), heading_rad),
        pos + rotate_vec(vec2(s * 0.42, s * 0.82), heading_rad),
        pos + rotate_vec(vec2(0.0, s * 0.75), heading_rad),
        pos + rotate_vec(vec2(-s * 0.42, s * 0.82), heading_rad),
    ];
    painter.add(egui::Shape::convex_polygon(tail, color, outline));

    // Fuselage spine
    let fuselage = vec![
        pos + rotate_vec(vec2(0.0, -s * 1.05), heading_rad),
        pos + rotate_vec(vec2(s * 0.16, -s * 0.45), heading_rad),
        pos + rotate_vec(vec2(s * 0.13, s * 0.85), heading_rad),
        pos + rotate_vec(vec2(-s * 0.13, s * 0.85), heading_rad),
        pos + rotate_vec(vec2(-s * 0.16, -s * 0.45), heading_rad),
    ];
    painter.add(egui::Shape::convex_polygon(fuselage, color, outline));
}

fn draw_dashed_polyline(
    painter: &egui::Painter,
    pts: &[Pos2],
    dash_len: f32,
    gap_len: f32,
    stroke: Stroke,
) {
    if pts.len() < 2 {
        return;
    }
    let step = dash_len + gap_len;
    for w in pts.windows(2) {
        let a = w[0];
        let b = w[1];
        let d = b - a;
        let len = d.length();
        if len < 0.5 {
            continue;
        }
        let dir = d / len;
        let mut t = 0.0;
        while t < len {
            let end_t = (t + dash_len).min(len);
            painter.line_segment([a + dir * t, a + dir * end_t], stroke);
            t += step;
        }
    }
}

fn draw_tactical_tooltip(painter: &egui::Painter, anchor: Pos2, lines: &[String]) {
    let max_chars = lines.iter().map(|s| s.chars().count()).max().unwrap_or(20);
    let w = (max_chars as f32 * 6.6 + 18.0).max(160.0);
    let h = lines.len() as f32 * 15.0 + 12.0;
    let rect = Rect::from_min_size(anchor, vec2(w, h));

    painter.rect_filled(
        rect,
        CornerRadius::same(4),
        Color32::from_rgba_unmultiplied(9, 14, 24, 242),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, Color32::from_rgb(56, 189, 248)),
        egui::StrokeKind::Outside,
    );

    for (idx, line) in lines.iter().enumerate() {
        let pos = rect.min + vec2(8.0, 6.0 + idx as f32 * 15.0);
        painter.text(
            pos,
            Align2::LEFT_TOP,
            line,
            FontId::monospace(10.5),
            if idx == 0 {
                Color32::from_rgb(251, 191, 36)
            } else {
                Color32::from_rgb(226, 232, 240)
            },
        );
    }
}

fn rotate_vec(v: Vec2, angle_rad: f32) -> Vec2 {
    let cos = angle_rad.cos();
    let sin = angle_rad.sin();
    vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

fn current_unix_secs() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
}

fn hms_from_epoch(epoch_secs: u64, offset_secs: u64) -> (u64, u64, u64) {
    let day_secs = (epoch_secs + offset_secs) % 86400;
    let h = day_secs / 3600;
    let m = (day_secs % 3600) / 60;
    let s = day_secs % 60;
    (h, m, s)
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not an HtmlCanvasElement");

        let web_options = eframe::WebOptions::default();
        if let Err(e) = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(RadarApp::new(cc)))),
            )
            .await
        {
            log::error!("Failed to start eframe: {:?}", e);
        }
    });
}

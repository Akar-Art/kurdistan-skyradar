//! Shared telemetry parsing and flight-enrichment engine.
//!
//! Used both by the standalone desktop app (`radar-app`) for direct in-process
//! telemetry ingestion and by `radar-server` for WebAssembly HTTP clients.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::atlas::Atlas;
use crate::db::{aircraft, airline_for, airport_city, is_military_or_gov};
use crate::geo::{bearing, haversine_nm};
use crate::model::{Flight, Snapshot, SourceStatus};
use crate::{COVERAGE_BOUNDS, KURDISTAN_CENTER};

/// One raw position report, normalised across feeds.
#[derive(Debug, Clone, Default)]
pub struct RawPosition {
    pub hex: String,
    pub flight_no: String,
    pub callsign: String,
    pub lat: f64,
    pub lon: f64,
    pub alt_ft: i32,
    pub on_ground: bool,
    pub speed_kts: f32,
    pub track: f32,
    pub vrate_fpm: i32,
    pub squawk: String,
    pub model: String,
    pub reg: String,
    pub origin: String,
    pub dest: String,
    /// Unix time of the position fix.
    pub fix_time: f64,
    pub source: &'static str,
}

pub fn fr24_feed_url() -> String {
    let (n, so, w, e) = COVERAGE_BOUNDS;
    format!(
        "https://data-cloud.flightradar24.com/zones/fcgi/feed.js?bounds={n},{so},{w},{e}&faa=1&satellite=1&mlat=1&flarm=1&adsb=1&gnd=1&air=1&vehicles=0&estimated=0&gliders=0&stats=0"
    )
}

pub fn adsb_feed_urls() -> [String; 2] {
    let (lat, lon) = KURDISTAN_CENTER;
    [
        format!("https://api.adsb.lol/v2/point/{lat}/{lon}/380"),
        format!("https://opendata.adsb.fi/api/v2/lat/{lat}/lon/{lon}/dist/280"),
    ]
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().trim().to_string()
}

pub fn parse_fr24_json(json_str: &str, now: f64) -> Option<Vec<RawPosition>> {
    let root: Value = serde_json::from_str(json_str).ok()?;
    let obj = root.as_object()?;

    let mut out = Vec::new();
    for v in obj.values() {
        let Some(a) = v.as_array() else { continue };
        if a.len() < 17 {
            continue;
        }
        let (Some(lat), Some(lon)) = (a[1].as_f64(), a[2].as_f64()) else { continue };
        let hex_raw = s(&a[0]).to_ascii_uppercase();
        let hex = if hex_raw.is_empty() {
            // Fallback key if ICAO24 hex is omitted on satellite/MLAT track
            let cs = s(&a[16]);
            if cs.is_empty() {
                continue;
            }
            cs
        } else {
            hex_raw
        };
        out.push(RawPosition {
            hex,
            lat,
            lon,
            track: a[3].as_f64().unwrap_or(0.0) as f32,
            alt_ft: a[4].as_i64().unwrap_or(0) as i32,
            speed_kts: a[5].as_f64().unwrap_or(0.0) as f32,
            squawk: s(&a[6]),
            model: s(&a[8]),
            reg: s(&a[9]),
            fix_time: a[10].as_f64().unwrap_or(now),
            origin: s(&a[11]),
            dest: s(&a[12]),
            flight_no: s(&a[13]),
            on_ground: a[14].as_i64().unwrap_or(0) == 1,
            vrate_fpm: a[15].as_i64().unwrap_or(0) as i32,
            callsign: s(&a[16]),
            source: "FR24",
        });
    }
    Some(out)
}

pub fn parse_adsb_json(json_str: &str, now: f64) -> Option<Vec<RawPosition>> {
    let root: Value = serde_json::from_str(json_str).ok()?;
    let list = root
        .get("ac")
        .or_else(|| root.get("aircraft"))
        .and_then(Value::as_array)?;
    Some(list.iter().filter_map(|a| parse_adsb_item(a, now)).collect())
}

fn parse_adsb_item(a: &Value, now: f64) -> Option<RawPosition> {
    let lat = a["lat"].as_f64()?;
    let lon = a["lon"].as_f64()?;
    let seen = a["seen_pos"].as_f64().unwrap_or(0.0);
    if seen > 60.0 {
        return None;
    }
    let (alt_ft, on_ground) = match &a["alt_baro"] {
        Value::String(g) if g == "ground" => (0, true),
        v => (v.as_f64().unwrap_or(0.0) as i32, false),
    };
    let vrate = a["baro_rate"]
        .as_f64()
        .or_else(|| a["geom_rate"].as_f64())
        .unwrap_or(0.0);
    let callsign = s(&a["flight"]);
    let hex = s(&a["hex"]).trim_start_matches('~').to_ascii_uppercase();
    if hex.is_empty() {
        return None;
    }
    Some(RawPosition {
        hex,
        flight_no: String::new(),
        callsign,
        lat,
        lon,
        alt_ft,
        on_ground,
        speed_kts: a["gs"].as_f64().unwrap_or(0.0) as f32,
        track: a["track"]
            .as_f64()
            .or_else(|| a["true_heading"].as_f64())
            .unwrap_or(0.0) as f32,
        vrate_fpm: vrate as i32,
        squawk: s(&a["squawk"]),
        model: s(&a["t"]),
        reg: s(&a["r"]),
        origin: String::new(),
        dest: String::new(),
        fix_time: now - seen,
        source: "ADS-B",
    })
}

/// Merges FR24 and ADS-B raw position lists, geofences every aircraft against
/// the Kurdistan Atlas, updates historical trails, and returns a `Snapshot`.
pub fn build_snapshot(
    atlas: &Atlas,
    fr24_res: Option<Vec<RawPosition>>,
    adsb_res: Option<Vec<RawPosition>>,
    trails: &mut HashMap<String, Vec<[f32; 2]>>,
    now: f64,
) -> Snapshot {
    let (ebl_lat, ebl_lon) = (36.2376, 43.9593);
    let (isu_lat, isu_lon) = (35.5614, 45.3182);

    let sources = SourceStatus {
        fr24: fr24_res.is_some(),
        adsb: adsb_res.is_some(),
    };

    let mut map: HashMap<String, RawPosition> = HashMap::new();

    if let Some(fr_list) = fr24_res {
        for f in fr_list {
            map.insert(f.hex.clone(), f);
        }
    }

    if let Some(adsb_list) = adsb_res {
        for a in adsb_list {
            if let Some(existing) = map.get_mut(&a.hex) {
                existing.lat = a.lat;
                existing.lon = a.lon;
                if a.speed_kts > 0.0 {
                    existing.speed_kts = a.speed_kts;
                }
                if a.track != 0.0 {
                    existing.track = a.track;
                }
                existing.alt_ft = a.alt_ft;
                existing.on_ground = a.on_ground;
                if a.vrate_fpm != 0 {
                    existing.vrate_fpm = a.vrate_fpm;
                }
                existing.fix_time = a.fix_time;
                if !a.squawk.is_empty() {
                    existing.squawk = a.squawk;
                }
                if !a.model.is_empty() && existing.model.is_empty() {
                    existing.model = a.model;
                }
                if !a.reg.is_empty() && existing.reg.is_empty() {
                    existing.reg = a.reg;
                }
                if !a.callsign.is_empty() && existing.callsign.is_empty() {
                    existing.callsign = a.callsign;
                }
                existing.source = "HYBRID";
            } else {
                map.insert(a.hex.clone(), a);
            }
        }
    }

    let mut flights = Vec::with_capacity(map.len());

    for (hex, raw) in map {
        let callsign_display = if !raw.flight_no.is_empty() {
            raw.flight_no.clone()
        } else if !raw.callsign.is_empty() {
            raw.callsign.clone()
        } else {
            hex.clone()
        };

        let is_military = is_military_or_gov(&callsign_display, &raw.model, &raw.squawk)
            || is_military_or_gov(&raw.callsign, &raw.model, &raw.squawk);

        let al = airline_for(&raw.flight_no)
            .or_else(|| airline_for(&raw.callsign))
            .or_else(|| airline_for(&callsign_display));

        let (airline, airline_country, airline_code) = match al {
            Some(a) => (a.name.to_string(), a.country.to_string(), a.iata.to_string()),
            None if is_military => (
                "Military / Gov Air Operations".to_string(),
                "Defense / Coalition".to_string(),
                "MIL".to_string(),
            ),
            None => (
                "Civil / Unscheduled".to_string(),
                "International".to_string(),
                callsign_display.chars().take(3).collect(),
            ),
        };

        let ac = aircraft(&raw.model);
        let (model_name, category, widebody) = match ac {
            Some(a) => (a.name.to_string(), a.category.to_string(), a.widebody),
            None if is_military => (
                if raw.model.is_empty() {
                    "Tactical / Gov Aircraft".to_string()
                } else {
                    raw.model.clone()
                },
                "Military / Gov".to_string(),
                false,
            ),
            None => (
                if raw.model.is_empty() {
                    "Transponder Track".to_string()
                } else {
                    raw.model.clone()
                },
                "Civil Aviation".to_string(),
                false,
            ),
        };

        let dist_ebl = haversine_nm(raw.lat, raw.lon, ebl_lat, ebl_lon) as f32;
        let dist_isu = haversine_nm(raw.lat, raw.lon, isu_lat, isu_lon) as f32;
        let radial_erbil = bearing(ebl_lat, ebl_lon, raw.lat, raw.lon) as f32;
        let radial_suly = bearing(isu_lat, isu_lon, raw.lat, raw.lon) as f32;

        let in_kri = atlas.in_kri(raw.lon, raw.lat);
        let disputed = atlas.disputed_at(raw.lon, raw.lat);
        let in_disputed = disputed.is_some();
        let province = atlas.province_at(raw.lon, raw.lat);
        let in_greater = in_kri || in_disputed || atlas.in_greater(raw.lon, raw.lat);
        let region = province.map(|p| p.label());
        let part = province.map(|p| p.part.clone());

        let status = if raw.on_ground || (raw.alt_ft <= 400 && raw.speed_kts < 45.0) {
            "ON GROUND".to_string()
        } else if raw.dest == "EBL" && dist_ebl <= 45.0 {
            format!("APPROACH HEWLÊR ({dist_ebl:.0} NM)")
        } else if raw.dest == "ISU" && dist_isu <= 45.0 {
            format!("APPROACH SILÊMANÎ ({dist_isu:.0} NM)")
        } else if raw.vrate_fpm > 350 {
            format!("CLIMBING FL{:03}", (raw.alt_ft / 100).max(0))
        } else if raw.vrate_fpm < -350 {
            format!("DESCENDING FL{:03}", (raw.alt_ft / 100).max(0))
        } else {
            format!("CRUISE FL{:03}", (raw.alt_ft / 100).max(0))
        };

        let trail_entry = trails.entry(hex.clone()).or_default();
        let pt = [raw.lat as f32, raw.lon as f32];
        let should_push = match trail_entry.last() {
            Some(last) => (last[0] - pt[0]).abs() > 0.0015 || (last[1] - pt[1]).abs() > 0.0015,
            None => true,
        };
        if should_push {
            trail_entry.push(pt);
            if trail_entry.len() > 36 {
                trail_entry.remove(0);
            }
        }
        let trail = trail_entry.clone();

        let pos_age_s = (now - raw.fix_time).clamp(0.0, 30.0) as f32;

        flights.push(Flight {
            hex,
            callsign: callsign_display,
            airline,
            airline_country,
            airline_code,
            model_code: raw.model,
            model_name,
            category,
            widebody,
            is_military,
            registration: raw.reg,
            lat: raw.lat,
            lon: raw.lon,
            alt_ft: raw.alt_ft,
            speed_kts: raw.speed_kts,
            heading: raw.track,
            vrate_fpm: raw.vrate_fpm,
            squawk: raw.squawk,
            on_ground: raw.on_ground,
            origin_city: airport_city(&raw.origin),
            origin: raw.origin,
            dest_city: airport_city(&raw.dest),
            dest: raw.dest,
            in_kri,
            in_disputed,
            in_greater,
            region,
            part,
            dist_erbil_nm: dist_ebl,
            dist_suly_nm: dist_isu,
            radial_erbil,
            radial_suly,
            status,
            source: raw.source.to_string(),
            pos_age_s,
            trail,
        });
    }

    let active_hexes: HashSet<String> = flights.iter().map(|f| f.hex.clone()).collect();
    trails.retain(|k, _| active_hexes.contains(k));

    Snapshot {
        timestamp: now,
        flights,
        sources,
    }
}

//! Data types exchanged between the telemetry engine and the radar UI.

use serde::{Deserialize, Serialize};

/// A single, fully enriched aircraft track.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Flight {
    /// ICAO 24-bit address, upper-case hex.
    pub hex: String,
    /// Flight number / callsign shown on the label (e.g. `TK771`).
    pub callsign: String,
    pub airline: String,
    pub airline_country: String,
    /// Short code for the badge (IATA when known).
    pub airline_code: String,
    /// ICAO aircraft type designator (e.g. `B38M`).
    pub model_code: String,
    pub model_name: String,
    pub category: String,
    pub widebody: bool,
    #[serde(default)]
    pub is_military: bool,
    pub registration: String,

    pub lat: f64,
    pub lon: f64,
    pub alt_ft: i32,
    pub speed_kts: f32,
    /// True track in degrees (0 = north, clockwise).
    pub heading: f32,
    pub vrate_fpm: i32,
    pub squawk: String,
    pub on_ground: bool,

    pub origin: String,
    pub origin_city: String,
    pub dest: String,
    pub dest_city: String,

    /// Inside the Kurdistan Region of Iraq (`Herêma Kurdistanê`).
    pub in_kri: bool,
    /// Inside an Article 140 or Rojava contested Kurdish district.
    #[serde(default)]
    pub in_disputed: bool,
    /// Inside any of the four parts of Greater Kurdistan (`Kurdistana Mezin`).
    pub in_greater: bool,
    /// Name of the Kurdish province or district the aircraft is over, if any.
    pub region: Option<String>,
    /// Part of Kurdistan (`Bakur`, `Başûr`, `Rojava`, `Rojhilat`), if over Kurdistan.
    #[serde(default)]
    pub part: Option<String>,

    pub dist_erbil_nm: f32,
    pub dist_suly_nm: f32,
    /// Bearing from Erbil airport to the aircraft.
    pub radial_erbil: f32,
    /// Bearing from Sulaymaniyah airport to the aircraft.
    pub radial_suly: f32,

    pub status: String,
    pub source: String,
    /// Age of the position fix in seconds at the moment the snapshot was built.
    pub pos_age_s: f32,
    /// Recent positions as `[lat, lon]`, oldest first.
    pub trail: Vec<[f32; 2]>,
}

impl Flight {
    /// True if squawking an international emergency code (`7700`, `7600`, `7500`).
    pub fn is_emergency(&self) -> bool {
        matches!(self.squawk.as_str(), "7700" | "7600" | "7500")
    }
}

/// Health of each upstream feed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SourceStatus {
    pub fr24: bool,
    pub adsb: bool,
}

/// Everything the frontend needs for one telemetry cycle.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    /// Unix time (seconds) when the snapshot was produced.
    pub timestamp: f64,
    pub flights: Vec<Flight>,
    pub sources: SourceStatus,
}

impl Snapshot {
    pub fn kri_count(&self) -> usize {
        self.flights.iter().filter(|f| f.in_kri).count()
    }

    pub fn greater_count(&self) -> usize {
        self.flights.iter().filter(|f| f.in_greater).count()
    }

    pub fn military_count(&self) -> usize {
        self.flights.iter().filter(|f| f.is_military).count()
    }

    /// Flights arriving at or departing from KRI / Kurdish hubs (`EBL`, `ISU`, `KIK`, `DHK`).
    pub fn kurdish_airport_count(&self) -> usize {
        self.flights
            .iter()
            .filter(|f| {
                ["EBL", "ISU", "KIK", "DHK"].contains(&f.origin.as_str())
                    || ["EBL", "ISU", "KIK", "DHK"].contains(&f.dest.as_str())
            })
            .count()
    }
}

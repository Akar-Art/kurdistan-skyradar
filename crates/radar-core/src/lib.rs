//! Kurdistan SkyRadar core library.
//!
//! Platform-independent models, geodesy, Kurdistan Atlas boundaries,
//! aviation reference tables, and telemetry enrichment shared between
//! the native PC application (`radar-app`), WebAssembly client, and
//! telemetry server (`radar-server`).

pub mod atlas;
pub mod db;
pub mod geo;
pub mod ingest;
pub mod model;

pub use atlas::{Atlas, Place, PlaceCategory, Region, RegionKind};
pub use model::{Flight, Snapshot, SourceStatus};

/// Default map centre (Hewlêr / Erbil) as `(lat, lon)`.
pub const KURDISTAN_CENTER: (f64, f64) = (36.85, 43.35);

/// Area polled from live feeds as `(north, south, west, east)`, covering
/// all four parts of Kurdistan (Bakur, Başûr, Rojava, Rojhilat) & Luristan.
pub const COVERAGE_BOUNDS: (f64, f64, f64, f64) = (41.4, 31.0, 35.8, 50.5);

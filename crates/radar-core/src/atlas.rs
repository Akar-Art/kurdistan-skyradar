//! The Kurdistan Atlas: official boundaries, 4-part division lines, disputed
//! territories (Article 140 & Rojava), ADM1 provinces, major lakes, and
//! curated Kurdish cities, capitals, and mountain peaks.
//!
//! Compiled from `atlas.kurd.sh` and geoBoundaries ADM1 into
//! `data/kurdistan_atlas.geojson`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::geo;

/// Built-in Kurdistan Atlas JSON embedded at compile time so the standalone
/// desktop executable and WASM bundle always have full map data available.
pub const BUILTIN_ATLAS_JSON: &str = include_str!("../../../data/kurdistan_atlas.geojson");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionKind {
    /// Continuous outer perimeter of Greater Kurdistan (`Kurdistana Mezin` / `Luristan`).
    GreaterOutline,
    /// Governorates of the Kurdistan Region of Iraq (`Herêma Kurdistanê`).
    Kri,
    /// UNAMI Article 140 disputed territories & Rojava contested districts.
    Disputed,
    /// Individual ADM1 provinces across Bakur, Başûr, Rojava, and Rojhilat.
    Greater,
    /// Major lakes and reservoirs inside Kurdistan (`Gola Wanê`, `Gola Urmiyê`, etc.).
    Lake,
}

/// One polygon or multipolygon region in the Kurdistan Atlas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub kurdish_name: Option<String>,
    pub part: String,
    pub kind: RegionKind,
    /// Control status for disputed territories (`"krg"`, `"federal"`, `"contested"`).
    pub control: Option<String>,
    pub label_hidden: bool,
    pub rings: Vec<Vec<[f64; 2]>>,
    /// `[min_lon, min_lat, max_lon, max_lat]` for fast bounding-box rejection.
    pub bbox: [f64; 4],
    /// Visual centroid `[lon, lat]` of the largest ring.
    pub center: [f64; 2],
}

impl Region {
    pub fn contains(&self, lon: f64, lat: f64) -> bool {
        let [x0, y0, x1, y1] = self.bbox;
        if lon < x0 || lon > x1 || lat < y0 || lat > y1 {
            return false;
        }
        self.rings.iter().any(|r| geo::point_in_ring(lon, lat, r))
    }

    /// Primary Kurdish display name (falls back to `name`).
    pub fn display_name(&self) -> &str {
        self.kurdish_name.as_deref().unwrap_or(&self.name)
    }

    /// Label in the form `"Hewlêr (Erbil)"` or `"Kerkûk"`.
    pub fn label(&self) -> String {
        match &self.kurdish_name {
            Some(k) if !k.is_empty() && k != &self.name => format!("{k} ({})", self.name),
            _ => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaceCategory {
    RegionLabel,
    Capital,
    HistoricalCapital,
    City,
    Peak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub id: String,
    pub name_ku: String,
    pub name_ckb: Option<String>,
    pub region: String,
    pub category: PlaceCategory,
    pub lon: f64,
    pub lat: f64,
    pub elevation_m: Option<i32>,
    /// `0` = region header / capital, `1` = primary city, `2` = town / peak.
    pub priority: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Atlas {
    pub regions: Vec<Region>,
    /// Internal dividing lines between Bakur, Başûr, Rojava, and Rojhilat.
    pub part_borders: Vec<Vec<[f64; 2]>>,
    /// Curated Kurdish capitals, cities, peaks, and regional headers.
    pub places: Vec<Place>,
}

impl Atlas {
    /// Loads the embedded Kurdistan Atlas with default tactical precision.
    pub fn builtin() -> Self {
        Self::from_geojson_str(BUILTIN_ATLAS_JSON, 0.0).unwrap_or_default()
    }

    /// Parses the combined atlas GeoJSON. `tolerance` (degrees) controls
    /// Douglas–Peucker simplification; `0.0` keeps the pre-simplified coordinates.
    pub fn from_geojson_str(src: &str, tolerance: f64) -> Result<Self, serde_json::Error> {
        let root: Value = serde_json::from_str(src)?;
        let mut regions = Vec::new();

        for (key, kind) in [
            ("greater_outlines", RegionKind::GreaterOutline),
            ("kri_provinces", RegionKind::Kri),
            ("disputed_territories", RegionKind::Disputed),
            ("greater_kurdistan", RegionKind::Greater),
            ("lakes", RegionKind::Lake),
        ] {
            if let Some(feats) = root[key]["features"].as_array() {
                for feat in feats {
                    if let Some(region) = parse_feature(feat, kind, tolerance) {
                        regions.push(region);
                    }
                }
            }
        }

        let mut part_borders = Vec::new();
        if let Some(lines) = root["part_borders"].as_array() {
            for line in lines {
                if let Some(pts) = line.as_array() {
                    let seg: Vec<[f64; 2]> = pts
                        .iter()
                        .filter_map(|p| Some([p.get(0)?.as_f64()?, p.get(1)?.as_f64()?]))
                        .collect();
                    if seg.len() >= 2 {
                        part_borders.push(seg);
                    }
                }
            }
        }

        let mut places = Vec::new();
        if let Some(place_arr) = root["places"].as_array() {
            for p in place_arr {
                let Some(lon) = p["lon"].as_f64() else { continue };
                let Some(lat) = p["lat"].as_f64() else { continue };
                let name_ku = p["name_ku"].as_str().unwrap_or_default().to_string();
                if name_ku.is_empty() {
                    continue;
                }
                let category = match p["category"].as_str().unwrap_or("city") {
                    "region-label" => PlaceCategory::RegionLabel,
                    "capital" => PlaceCategory::Capital,
                    "historical-capital" => PlaceCategory::HistoricalCapital,
                    "peak" => PlaceCategory::Peak,
                    _ => PlaceCategory::City,
                };
                places.push(Place {
                    id: p["id"].as_str().unwrap_or_default().to_string(),
                    name_ku,
                    name_ckb: p["name_ckb"].as_str().map(str::to_string),
                    region: p["region"].as_str().unwrap_or_default().to_string(),
                    category,
                    lon,
                    lat,
                    elevation_m: p["elevation_m"].as_i64().map(|v| v as i32),
                    priority: p["priority"].as_u64().unwrap_or(2) as u8,
                });
            }
        }

        Ok(Self {
            regions,
            part_borders,
            places,
        })
    }

    pub fn kri(&self) -> impl Iterator<Item = &Region> {
        self.regions.iter().filter(|r| r.kind == RegionKind::Kri)
    }

    pub fn greater_outlines(&self) -> impl Iterator<Item = &Region> {
        self.regions
            .iter()
            .filter(|r| r.kind == RegionKind::GreaterOutline)
    }

    /// Primary district-accurate outer boundary of Greater Kurdistan (`greater-akb`).
    pub fn primary_outline(&self) -> Option<&Region> {
        self.greater_outlines()
            .find(|r| r.id == "greater-akb")
            .or_else(|| self.greater_outlines().next())
    }

    /// Optional Luristan boundary in the southern Zagros.
    pub fn luristan_outline(&self) -> Option<&Region> {
        self.greater_outlines().find(|r| r.id == "luristan")
    }

    pub fn disputed(&self) -> impl Iterator<Item = &Region> {
        self.regions
            .iter()
            .filter(|r| r.kind == RegionKind::Disputed)
    }

    /// ADM1 provinces across all 4 parts of Kurdistan.
    pub fn greater(&self) -> impl Iterator<Item = &Region> {
        self.regions
            .iter()
            .filter(|r| r.kind == RegionKind::Greater)
    }

    pub fn lakes(&self) -> impl Iterator<Item = &Region> {
        self.regions.iter().filter(|r| r.kind == RegionKind::Lake)
    }

    /// True if `(lon, lat)` is inside the Kurdistan Region of Iraq (`Herêma Kurdistanê`),
    /// including KRG-administered areas.
    pub fn in_kri(&self, lon: f64, lat: f64) -> bool {
        self.kri().any(|r| r.contains(lon, lat))
            || self
                .disputed()
                .any(|r| r.control.as_deref() == Some("krg") && r.contains(lon, lat))
    }

    /// Returns the disputed/contested district at `(lon, lat)`, if any.
    pub fn disputed_at(&self, lon: f64, lat: f64) -> Option<&Region> {
        self.disputed().find(|r| r.contains(lon, lat))
    }

    /// True if `(lon, lat)` is anywhere inside Greater Kurdistan (`Kurdistana Mezin`).
    pub fn in_greater(&self, lon: f64, lat: f64) -> bool {
        self.greater_outlines().any(|r| r.contains(lon, lat))
            || self.kri().any(|r| r.contains(lon, lat))
            || self.disputed().any(|r| r.contains(lon, lat))
    }

    /// Resolves the specific Kurdish province or disputed district containing `(lon, lat)`.
    pub fn province_at(&self, lon: f64, lat: f64) -> Option<&Region> {
        // 1. Check KRI governorates first
        if let Some(r) = self.kri().find(|r| r.contains(lon, lat)) {
            return Some(r);
        }
        // 2. Check disputed Kurdish districts (Kerkûk, Şingal, Xaneqîn, Efrîn, Serê Kaniyê, Girê Spî, etc.)
        if let Some(r) = self.disputed().find(|r| r.contains(lon, lat)) {
            return Some(r);
        }
        // 3. Check ADM1 provinces if inside Greater Kurdistan
        if self.greater_outlines().any(|r| r.contains(lon, lat)) {
            if let Some(r) = self.greater().find(|r| r.contains(lon, lat)) {
                return Some(r);
            }
        }
        // 4. Fallback to core Kurdish ADM1 provinces
        self.greater().find(|r| r.contains(lon, lat))
    }

    pub fn point_count(&self) -> usize {
        self.regions.iter().flat_map(|r| &r.rings).map(Vec::len).sum()
    }
}

fn parse_feature(feat: &Value, kind: RegionKind, tolerance: f64) -> Option<Region> {
    let props = &feat["properties"];
    let geom = &feat["geometry"];
    let polys: Vec<&Value> = match geom["type"].as_str()? {
        "Polygon" => vec![&geom["coordinates"]],
        "MultiPolygon" => geom["coordinates"].as_array()?.iter().collect(),
        _ => return None,
    };

    let mut rings = Vec::new();
    for poly in polys {
        let outer = poly.as_array()?.first()?.as_array()?;
        let pts: Vec<[f64; 2]> = outer
            .iter()
            .filter_map(|p| Some([p.get(0)?.as_f64()?, p.get(1)?.as_f64()?]))
            .collect();
        let pts = if tolerance > 0.0 {
            geo::simplify(&pts, tolerance)
        } else {
            pts
        };
        if pts.len() >= 4 {
            rings.push(pts);
        }
    }
    if rings.is_empty() {
        return None;
    }

    let mut bbox = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for p in rings.iter().flatten() {
        bbox[0] = bbox[0].min(p[0]);
        bbox[1] = bbox[1].min(p[1]);
        bbox[2] = bbox[2].max(p[0]);
        bbox[3] = bbox[3].max(p[1]);
    }

    // Compute visual centroid of the largest ring
    let largest_ring = rings.iter().max_by_key(|r| r.len()).unwrap();
    let (mut sum_x, mut sum_y) = (0.0, 0.0);
    for &[x, y] in largest_ring {
        sum_x += x;
        sum_y += y;
    }
    let n = largest_ring.len() as f64;
    let center = [sum_x / n, sum_y / n];

    let name = props["name"].as_str().unwrap_or("Unknown").to_string();
    let id = props["id"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| name.to_lowercase());

    Some(Region {
        id,
        name,
        kurdish_name: props["kurdish_name"].as_str().map(str::to_string),
        part: props["part"].as_str().unwrap_or("Kurdistan").to_string(),
        kind,
        control: props["control"].as_str().map(str::to_string),
        label_hidden: props["label_hidden"].as_bool().unwrap_or(false),
        rings,
        bbox,
        center,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_atlas_loads_all_layers_and_places() {
        let atlas = Atlas::builtin();
        assert!(atlas.primary_outline().is_some(), "missing Greater Kurdistan outline");
        assert_eq!(atlas.kri().count(), 3, "expected 3 KRI governorates");
        assert!(atlas.disputed().count() >= 15, "expected disputed districts");
        assert!(atlas.greater().count() >= 35, "expected >=35 ADM1 provinces");
        assert!(!atlas.part_borders.is_empty(), "expected 4-part border lines");
        assert!(atlas.places.len() >= 100, "expected >=100 curated places");

        // Verify KRI capitals
        assert!(atlas.in_kri(44.009, 36.191), "Hewlêr must be in KRI");
        assert!(atlas.in_kri(45.436, 35.557), "Silêmanî must be in KRI");
        assert!(atlas.in_kri(42.949, 36.868), "Duhok must be in KRI");

        // Verify previously missing regions are now in Greater Kurdistan
        assert!(atlas.in_greater(41.277, 39.909), "Erzirom must be in Greater Kurdistan");
        assert!(atlas.in_greater(39.490, 39.750), "Erzingan must be in Greater Kurdistan");
        assert!(atlas.in_greater(36.923, 37.575), "Meraş must be in Greater Kurdistan");
        assert!(atlas.in_greater(36.866, 36.512), "Efrîn must be in Greater Kurdistan");
        assert!(atlas.in_greater(38.354, 36.891), "Kobanî must be in Greater Kurdistan");
        assert!(atlas.in_greater(41.221, 37.052), "Qamişlo must be in Greater Kurdistan");
        assert!(atlas.in_greater(44.380, 35.466), "Kerkûk must be in Greater Kurdistan");
        assert!(atlas.in_greater(41.876, 36.322), "Şingal must be in Greater Kurdistan");
        assert!(atlas.in_greater(46.996, 35.311), "Sine must be in Greater Kurdistan");
        assert!(atlas.in_greater(47.065, 34.314), "Kirmaşan must be in Greater Kurdistan");
    }
}

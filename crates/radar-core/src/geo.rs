//! Geodesy helpers: distances, bearings, dead reckoning, polygon tests and
//! line simplification.

const EARTH_RADIUS_KM: f64 = 6371.0;
const EARTH_RADIUS_NM: f64 = 3440.065;

/// Great-circle distance in kilometres.
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    central_angle(lat1, lon1, lat2, lon2) * EARTH_RADIUS_KM
}

/// Great-circle distance in nautical miles.
pub fn haversine_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    central_angle(lat1, lon1, lat2, lon2) * EARTH_RADIUS_NM
}

fn central_angle(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    2.0 * a.sqrt().atan2((1.0 - a).sqrt())
}

/// Initial bearing in degrees `[0, 360)` from point 1 to point 2.
pub fn bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
}

/// Moves a point `distance_nm` along `heading_deg` (great-circle destination).
pub fn destination(lat: f64, lon: f64, heading_deg: f64, distance_nm: f64) -> (f64, f64) {
    let d = distance_nm / EARTH_RADIUS_NM;
    let h = heading_deg.to_radians();
    let p1 = lat.to_radians();
    let l1 = lon.to_radians();
    let p2 = (p1.sin() * d.cos() + p1.cos() * d.sin() * h.cos()).asin();
    let l2 = l1 + (h.sin() * d.sin() * p1.cos()).atan2(d.cos() - p1.sin() * p2.sin());
    (p2.to_degrees(), l2.to_degrees())
}

/// Predicts where an aircraft is after `dt_s` seconds at constant speed and track.
pub fn dead_reckon(lat: f64, lon: f64, heading_deg: f64, speed_kts: f64, dt_s: f64) -> (f64, f64) {
    if speed_kts <= 1.0 || dt_s <= 0.0 {
        return (lat, lon);
    }
    destination(lat, lon, heading_deg, speed_kts * dt_s / 3600.0)
}

/// Shortest signed difference `b - a` between two angles, in `(-180, 180]`.
pub fn angle_diff(a: f64, b: f64) -> f64 {
    let mut d = (b - a) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d <= -180.0 {
        d += 360.0;
    }
    d
}

/// 16-point compass name for a bearing.
pub fn cardinal(deg: f64) -> &'static str {
    const DIRS: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW",
    ];
    DIRS[((deg.rem_euclid(360.0) / 22.5).round() as usize) % 16]
}

/// Ray-casting point-in-polygon test. `ring` holds `[lon, lat]` pairs.
pub fn point_in_ring(lon: f64, lat: f64, ring: &[[f64; 2]]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let [xi, yi] = ring[i];
        let [xj, yj] = ring[j];
        if (yi > lat) != (yj > lat) && lon < (xj - xi) * (lat - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Douglas–Peucker simplification (planar, tolerance in degrees).
/// Keeps the first and last point so closed rings stay closed.
pub fn simplify(points: &[[f64; 2]], tolerance: f64) -> Vec<[f64; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0usize, points.len() - 1)];
    let tol2 = tolerance * tolerance;

    while let Some((start, end)) = stack.pop() {
        let mut max_d = 0.0;
        let mut idx = start;
        for i in start + 1..end {
            let d = perpendicular_dist2(points[i], points[start], points[end]);
            if d > max_d {
                max_d = d;
                idx = i;
            }
        }
        if max_d > tol2 {
            keep[idx] = true;
            stack.push((start, idx));
            stack.push((idx, end));
        }
    }

    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

fn perpendicular_dist2(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return (p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0);
    let (cx, cy) = (a[0] + t * dx, a[1] + t * dy);
    (p[0] - cx).powi(2) + (p[1] - cy).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erbil_to_sulaymaniyah_distance() {
        let km = haversine_km(36.2376, 43.9593, 35.5614, 45.3182);
        assert!((km - 141.0).abs() < 5.0, "got {km}");
    }

    #[test]
    fn dead_reckoning_moves_north() {
        let (lat, lon) = dead_reckon(36.0, 44.0, 0.0, 600.0, 60.0); // 10 NM north
        assert!((lat - 36.1667).abs() < 0.01);
        assert!((lon - 44.0).abs() < 1e-6);
    }

    #[test]
    fn angle_wraps() {
        assert_eq!(angle_diff(350.0, 10.0), 20.0);
        assert_eq!(angle_diff(10.0, 350.0), -20.0);
    }

    #[test]
    fn square_contains_centre() {
        let sq = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]];
        assert!(point_in_ring(0.5, 0.5, &sq));
        assert!(!point_in_ring(1.5, 0.5, &sq));
    }
}

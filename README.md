# ✈️ Kurdistan SkyRadar C2 (Asoya Esmanî ya Kurdistanê)

An ultra-responsive, standalone 100% Rust Airspace Command & Control (C2) desktop application and high-precision radar covering all four parts of Kurdistan (**Bakur, Başûr, Rojava, Rojhilat**) and the Kurdistan Region (**Herêma Kurdistanê**).

![Status](https://img.shields.io/badge/Status-Government%20%2F%20C2%20Grade-10b981?style=for-the-badge)
![Language](https://img.shields.io/badge/Language-100%25%20Rust-e05d44?style=for-the-badge)
![GUI](https://img.shields.io/badge/GUI-egui%200.36%20%2B%20walkers-6366f1?style=for-the-badge)
![Platform](https://img.shields.io/badge/Platform-Windows%20Native%20%7C%20WASM-0ea5e9?style=for-the-badge)

---

## 🌟 Key Highlights

1. **100% Rust Native PC Application**
   - Built with `eframe`, `egui`, and `walkers`.
   - Compiles to a self-contained, high-performance Windows executable (`KurdistanSkyRadar.exe`) with native hardware acceleration.
   - Built-in asynchronous background telemetry worker (Tokio + `reqwest`) with concurrent multi-source ingestion from **Flightradar24** and **airplanes.live (ADS-B Exchange)**.

2. **Official Kurdistan Atlas Integration (`atlas.kurd.sh`)**
   - District-accurate continuous outer boundary of **Greater Kurdistan** (`greater-akb`) and **Luristan / Southern Zagros** (`luristan`).
   - Internal 4-part dividing lines separating **Bakur**, **Başûr**, **Rojava**, and **Rojhilat**.
   - Full 41 ADM1 provinces with complete Kurdish toponymy.
   - 24 UNAMI Article 140 & Rojava contested district polygons (Kerkûk, Şingal, Nineveh Plains, Mexmûr, Xaneqîn, Efrîn, etc.).
   - Major Kurdish water bodies (Lake Van / Gola Wanê, Lake Urmia / Gola Urmiyê, Lake Dukan / Gola Dûkanê, etc.).
   - 164 curated Kurdish places (capitals, historical capitals, cities, mountain peaks) and 31 Kurdish airports across all four parts.

3. **Zero-Jitter Flight Kinematics**
   - Per-aircraft `VisualTrack` state maintaining continuous frame-delta dead-reckoning.
   - Smooth exponential interpolation eliminates position snapping when telemetry packets arrive.
   - 60-second velocity vector leader lines and swept-wing aircraft silhouettes rendered with crisp vector primitives.

4. **Tactical Airspace Command & Control (C2) Interface**
   - 42px top tactical command bar with live squawk alert indicators (7700/7600/7500), regional counters, and layer filters.
   - Single collapsible Left Tactical Console (340px) with high-density monospace track table and target dossier.
   - Camera tracking lock (`LOCK CAMERA`), callsign search, and military / government (`MIL / GOV`) classification.

---

## 🏗️ Architecture

```
kurdistan-radar/
├── crates/
│   ├── radar-core/     # Pure Rust domain logic, geofencing, DB, ingest parsers, embedded atlas
│   ├── radar-server/   # Headless Axum telemetry daemon and REST / SSE server
│   └── radar-app/      # Standalone native C2 desktop application + WebAssembly target
├── data/
│   └── kurdistan_atlas.geojson  # Optimized official multi-layer Kurdistan boundaries & places
└── Cargo.toml          # Cargo workspace
```

---

## 🚀 Building & Running

### Prerequisites
- [Rust & Cargo](https://rustup.rs/) (stable toolchain)
- Windows (native MSVC toolchain) or Linux / macOS

### Native Desktop App
```bash
cargo run --release -p radar-app
```
*Or build the standalone executable:*
```bash
cargo build --release -p radar-app
```

### Headless Server (Optional)
```bash
cargo run --release -p radar-server
```

---

## 🗺️ Airspace Coverage
- **Center**: `36.5° N, 44.0° E`
- **Coverage**: Lat `31.0° N - 41.4° N`, Lon `35.8° E - 50.5° E` (all four parts of Kurdistan)

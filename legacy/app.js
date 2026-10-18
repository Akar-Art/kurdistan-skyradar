/**
 * Kurdistan SkyRadar - Frontend Application
 * Real-time ADS-B and FlightRadar24 air traffic tracking over Kurdistan
 */

// Application State
const state = {
  flights: {},              // hex -> flight data
  markers: {},              // hex -> Leaflet marker
  trails: {},               // hex -> Leaflet polyline
  selectedHex: null,        // Currently inspected aircraft hex
  followingHex: null,       // Currently camera-locked aircraft hex
  filter: 'all',            // all, kri, erbil, suly, cruise, low
  searchQuery: '',
  audioEnabled: false,
  showRadarSweep: true,
  showTrails: true,
  showBorders: true,
  showAirports: true,
  simulationMode: false,
  nextPollCountdown: 4,
  mapStyle: 'dark',
  airports: {},
  boundaries: null
};

// Colors by Altitude (Feet)
function getAltitudeColor(altFt) {
  if (altFt <= 500) return '#64748b';        // Ground / Slate
  if (altFt < 10000) return '#10b981';       // Low / Emerald
  if (altFt < 25000) return '#06b6d4';       // Mid / Cyan
  if (altFt < 36000) return '#3b82f6';       // Cruise / Blue
  return '#a855f7';                          // High / Violet
}

// Generate Plane SVG Icon with Altitude Color and Heading Rotation
function createPlaneIcon(flight, isSelected = false) {
  const color = isSelected ? '#fbbf24' : getAltitudeColor(flight.altitude_ft);
  const size = flight.widebody ? 30 : (flight.category === 'Turboprop' ? 22 : 26);
  const heading = flight.heading || 0;
  const callsign = flight.flight || flight.hex;

  const svg = `
    <div class="plane-marker-icon ${isSelected ? 'selected' : ''}" style="width: ${size}px; height: ${size}px;">
      <svg class="plane-svg" viewBox="0 0 24 24" width="${size}" height="${size}" style="transform: rotate(${heading}deg);">
        <path fill="${color}" stroke="#07090e" stroke-width="1.2" d="M21 16v-2l-8-5V3.5c0-.83-.67-1.5-1.5-1.5S10 2.67 10 3.5V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1 3.5 1v-1.5L13 19v-5.5l8 2.5z"/>
      </svg>
      <div class="plane-marker-label" style="display: ${isSelected || flight.is_kri ? 'block' : 'none'};">
        ${callsign}
      </div>
    </div>
  `;

  return L.divIcon({
    html: svg,
    className: 'custom-plane-icon',
    iconSize: [size, size],
    iconAnchor: [size / 2, size / 2]
  });
}

// Web Audio API Synthesizer for Tactical ATC Radar Sounds
const audioCtx = new (window.AudioContext || window.webkitAudioContext)();
function playRadarBlip(pitch = 880, duration = 0.08) {
  if (!state.audioEnabled) return;
  try {
    if (audioCtx.state === 'suspended') audioCtx.resume();
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(pitch, audioCtx.currentTime);
    osc.frequency.exponentialRampToValueAtTime(pitch * 0.5, audioCtx.currentTime + duration);
    gain.gain.setValueAtTime(0.12, audioCtx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, audioCtx.currentTime + duration);
    osc.connect(gain);
    gain.connect(audioCtx.destination);
    osc.start();
    osc.stop(audioCtx.currentTime + duration);
  } catch (e) {
    console.error("Audio error", e);
  }
}

// Initialize Leaflet Map
let map;
let baseTileLayers = {};
let borderLayerGroup = L.layerGroup();
let airportLayerGroup = L.layerGroup();
let trailLayerGroup = L.layerGroup();
let planeLayerGroup = L.layerGroup();

function initMap() {
  // Center: Erbil / Kurdistan Region
  const kurdistanCenter = [36.2376, 44.0];
  map = L.map('map', {
    center: kurdistanCenter,
    zoom: 8,
    minZoom: 6,
    maxZoom: 16,
    zoomControl: false,
    attributionControl: false
  });

  // Re-position zoom control to bottom right
  L.control.zoom({ position: 'bottomright' }).addTo(map);

  // Tile Providers (100% Free, NO API Key Required, NO Watermarks)
  baseTileLayers.dark = L.layerGroup([
    L.tileLayer('https://services.arcgisonline.com/arcgis/rest/services/Canvas/World_Dark_Gray_Base/MapServer/tile/{z}/{y}/{x}', {
      maxZoom: 16,
      attribution: 'Esri, OpenStreetMap'
    }),
    L.tileLayer('https://services.arcgisonline.com/arcgis/rest/services/Canvas/World_Dark_Gray_Reference/MapServer/tile/{z}/{y}/{x}', {
      maxZoom: 16
    })
  ]);

  baseTileLayers.midnight = L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
    maxZoom: 19,
    className: 'osm-dark-tiles',
    attribution: '© OpenStreetMap contributors'
  });

  baseTileLayers.satellite = L.layerGroup([
    L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}', {
      maxZoom: 18,
      attribution: 'Esri, Maxar'
    }),
    L.tileLayer('https://services.arcgisonline.com/arcgis/rest/services/Canvas/World_Dark_Gray_Reference/MapServer/tile/{z}/{y}/{x}', {
      maxZoom: 18
    })
  ]);

  baseTileLayers.dark.addTo(map);

  // Add Layer Groups
  borderLayerGroup.addTo(map);
  airportLayerGroup.addTo(map);
  trailLayerGroup.addTo(map);
  planeLayerGroup.addTo(map);

  // Map click deselects unless clicking marker
  map.on('click', () => {
    if (state.followingHex) stopFollowing();
  });
}

// Load Kurdistan Atlas GeoJSON Boundaries
async function loadBoundaries() {
  try {
    const res = await fetch('/api/boundaries');
    const data = await res.json();
    state.boundaries = data;
    renderBoundaries();
  } catch (err) {
    console.error("Failed to load borders:", err);
  }
}

function renderBoundaries() {
  borderLayerGroup.clearLayers();
  if (!state.showBorders || !state.boundaries) return;

  // 1. Greater Kurdistan Atlas (All 4 Parts: Bakur, Başûr, Rojava, Rojhilat)
  if (state.boundaries.greater_kurdistan) {
    L.geoJSON(state.boundaries.greater_kurdistan, {
      style: () => ({
        color: '#00f2fe',
        weight: 1.4,
        opacity: 0.65,
        fillColor: '#00f2fe',
        fillOpacity: 0.03,
        dashArray: '3, 5'
      }),
      onEachFeature: (feat, layer) => {
        const p = feat.properties || {};
        const krd = p.kurdish_name ? `<strong style="color: #00f2fe;">${p.kurdish_name}</strong> &bull; ` : '';
        layer.bindTooltip(`
          <div class="greater-leaflet-tooltip">
            <div>${krd}${p.name}</div>
            <div style="font-size: 9px; color: #94a3b8; margin-top: 2px;">${p.part || 'Kurdistan Atlas'}</div>
          </div>
        `, { direction: 'top', sticky: true });

        layer.on('mouseover', (e) => {
          e.target.setStyle({ weight: 2.4, opacity: 1, fillOpacity: 0.09 });
        });
        layer.on('mouseout', (e) => {
          e.target.setStyle({ weight: 1.4, opacity: 0.65, fillOpacity: 0.03 });
        });
      }
    }).addTo(borderLayerGroup);
  }

  // 2. Kurdistan Region of Iraq (KRI / Başûr) - Highlighted in Kurdish Sun Gold
  if (state.boundaries.kri_provinces) {
    L.geoJSON(state.boundaries.kri_provinces, {
      style: () => ({
        color: '#fbbf24',
        weight: 2.8,
        opacity: 0.95,
        fillColor: '#fbbf24',
        fillOpacity: 0.1,
        dashArray: '6, 4'
      }),
      onEachFeature: (feat, layer) => {
        const p = feat.properties || {};
        layer.bindTooltip(`
          <div class="kri-leaflet-tooltip">
            <div><strong style="color: #fbbf24;">☀️ ${p.kurdish_name || p.name}</strong> (${p.name})</div>
            <div style="font-size: 9px; color: #f59e0b; margin-top: 2px;">Kurdistan Region of Iraq (KRI)</div>
          </div>
        `, { direction: 'top', sticky: true });

        layer.on('mouseover', (e) => {
          e.target.setStyle({ weight: 3.8, opacity: 1, fillOpacity: 0.18 });
        });
        layer.on('mouseout', (e) => {
          e.target.setStyle({ weight: 2.8, opacity: 0.95, fillOpacity: 0.1 });
        });
      }
    }).addTo(borderLayerGroup);
  }
}

// Load and Plot Kurdistan and Regional Airports
async function loadAirports() {
  try {
    const res = await fetch('/api/airports');
    const data = await res.json();
    state.airports = data;
    renderAirports();
  } catch (err) {
    console.error("Failed to load airports:", err);
  }
}

function renderAirports() {
  airportLayerGroup.clearLayers();
  if (!state.showAirports || !state.airports) return;

  Object.entries(state.airports).forEach(([code, apt]) => {
    if (!apt.lat || !apt.lon) return;

    const isKurdishHub = apt.is_kurdistan;
    const markerColor = isKurdishHub ? '#fbbf24' : '#64748b';

    const iconHtml = `
      <div class="airport-beacon" title="${apt.name} (${code})">
        <div class="airport-beacon-center" style="background: ${markerColor}; box-shadow: 0 0 8px ${markerColor};"></div>
        ${isKurdishHub ? `<div class="airport-beacon-pulse" style="border-color: ${markerColor};"></div>` : ''}
      </div>
    `;

    const icon = L.divIcon({
      html: iconHtml,
      className: 'custom-airport-icon',
      iconSize: [24, 24],
      iconAnchor: [12, 12]
    });

    const marker = L.marker([apt.lat, apt.lon], { icon }).addTo(airportLayerGroup);
    
    // Tooltip with airport name and elevation
    marker.bindTooltip(`
      <div class="airport-marker-label">
        <span style="color: ${markerColor}; font-weight: 700;">✈ ${apt.iata || code}</span> &bull; ${apt.city}
      </div>
    `, {
      permanent: isKurdishHub,
      direction: 'top',
      offset: [0, -12],
      className: 'airport-leaflet-tooltip'
    });

    marker.on('click', () => {
      map.flyTo([apt.lat, apt.lon], 11, { duration: 1.2 });
      playRadarBlip(1200, 0.1);
    });
  });
}

// Poll Real-Time Flight Data
async function pollFlights() {
  try {
    const res = await fetch('/api/flights');
    const data = await res.json();
    processFlightData(data);
  } catch (err) {
    console.warn("Polling retry...", err);
    document.getElementById('backend-status-text').textContent = 'Reconnecting to live ADS-B telemetry stream...';
  }
}

// Process and Render Live Flight Data
function processFlightData(data) {
  const flightList = data.flights || [];
  const activeHexes = new Set();
  const now = Date.now();

  // Update Top Stats
  document.getElementById('stat-total').textContent = data.total_flights || 0;
  document.getElementById('stat-kri').textContent = data.kri_airspace_flights || 0;
  document.getElementById('stat-airports').textContent = (data.erbil_flights || 0) + (data.sulaymaniyah_flights || 0);

  // Update Simulation Button Status
  const simBtn = document.getElementById('btn-toggle-sim');
  if (data.simulation_mode) {
    simBtn.classList.add('active');
  } else {
    simBtn.classList.remove('active');
  }
  state.simulationMode = data.simulation_mode;

  // Process Each Flight
  flightList.forEach(flight => {
    const hex = flight.hex;
    activeHexes.add(hex);

    const prev = state.flights[hex];
    state.flights[hex] = flight;

    // Smooth Dead Reckoning Marker Movement
    if (!state.markers[hex]) {
      // Create new aircraft marker
      const marker = L.marker([flight.lat, flight.lon], {
        icon: createPlaneIcon(flight, hex === state.selectedHex),
        zIndexOffset: flight.is_kri ? 1000 : 500
      }).addTo(planeLayerGroup);

      marker.on('click', () => selectFlight(hex));
      state.markers[hex] = marker;
    } else {
      // Smoothly update marker position and icon rotation
      const marker = state.markers[hex];
      marker.setLatLng([flight.lat, flight.lon]);
      marker.setIcon(createPlaneIcon(flight, hex === state.selectedHex));
    }

    // Render Trail
    renderTrail(hex, flight);
  });

  // Remove Extinct / Departed Flights
  Object.keys(state.markers).forEach(hex => {
    if (!activeHexes.has(hex)) {
      planeLayerGroup.removeLayer(state.markers[hex]);
      delete state.markers[hex];
      if (state.trails[hex]) {
        trailLayerGroup.removeLayer(state.trails[hex]);
        delete state.trails[hex];
      }
      delete state.flights[hex];

      if (state.selectedHex === hex) {
        closeDrawer();
      }
    }
  });

  // If Following Plane, Pan Camera
  if (state.followingHex && state.flights[state.followingHex]) {
    const f = state.flights[state.followingHex];
    map.panTo([f.lat, f.lon], { animate: true, duration: 0.8 });
  }

  // Update Flight Sidebar Cards
  renderFlightList();

  // If Flight Inspector Drawer is open, refresh live telemetry numbers
  if (state.selectedHex && state.flights[state.selectedHex]) {
    populateDrawer(state.flights[state.selectedHex]);
  }

  // Update Footer Status
  document.getElementById('backend-status-text').textContent = 
    `Active Stream: ${flightList.length} aircraft tracked (${data.kri_airspace_flights} over Kurdistan)`;
  document.getElementById('data-updated-time').textContent = `Updated: ${new Date().toLocaleTimeString()}`;
}

// Render Breadcrumb Flight Trail
function renderTrail(hex, flight) {
  if (!state.showTrails) {
    if (state.trails[hex]) {
      trailLayerGroup.removeLayer(state.trails[hex]);
      delete state.trails[hex];
    }
    return;
  }

  const trailCoords = (flight.trail || []).map(p => [p[0], p[1]]);
  if (trailCoords.length < 2) return;

  const isSelected = hex === state.selectedHex;
  const color = isSelected ? '#fbbf24' : getAltitudeColor(flight.altitude_ft);

  if (!state.trails[hex]) {
    const poly = L.polyline(trailCoords, {
      color: color,
      weight: isSelected ? 3 : 1.8,
      opacity: isSelected ? 0.9 : 0.45,
      dashArray: '3, 6'
    }).addTo(trailLayerGroup);
    state.trails[hex] = poly;
  } else {
    state.trails[hex].setLatLngs(trailCoords);
    state.trails[hex].setStyle({
      color: color,
      weight: isSelected ? 3 : 1.8,
      opacity: isSelected ? 0.9 : 0.45
    });
  }
}

// Filter and Render Flight Cards in Sidebar
function renderFlightList() {
  const container = document.getElementById('flight-list');
  const query = state.searchQuery.toLowerCase();
  
  const allFlights = Object.values(state.flights);
  const filtered = allFlights.filter(f => {
    // Category filter
    if (state.filter === 'kri' && !f.is_kri) return false;
    if (state.filter === 'erbil' && f.dest_iata !== 'EBL' && f.origin_iata !== 'EBL') return false;
    if (state.filter === 'suly' && f.dest_iata !== 'ISU' && f.origin_iata !== 'ISU') return false;
    if (state.filter === 'cruise' && f.altitude_ft < 30000) return false;
    if (state.filter === 'low' && f.altitude_ft > 15000) return false;

    // Search query
    if (query) {
      const matchCallsign = (f.flight || '').toLowerCase().includes(query);
      const matchAirline = (f.airline || '').toLowerCase().includes(query);
      const matchModel = (f.aircraft_model || '').toLowerCase().includes(query);
      const matchOrig = (f.origin_city || f.origin_iata || '').toLowerCase().includes(query);
      const matchDest = (f.dest_city || f.dest_iata || '').toLowerCase().includes(query);
      const matchReg = (f.registration || '').toLowerCase().includes(query);
      return matchCallsign || matchAirline || matchModel || matchOrig || matchDest || matchReg;
    }
    return true;
  });

  // Sort: Kurdistan flights first, then by altitude
  filtered.sort((a, b) => {
    if (a.is_kri && !b.is_kri) return -1;
    if (!a.is_kri && b.is_kri) return 1;
    return b.altitude_ft - a.altitude_ft;
  });

  document.getElementById('filtered-count-label').textContent = `Showing ${filtered.length} of ${allFlights.length} flights`;

  if (filtered.length === 0) {
    container.innerHTML = `
      <div class="flight-list-loading">
        <i data-lucide="plane-slash" style="width: 32px; height: 32px; color: var(--text-dim);"></i>
        <span>No aircraft matching filters</span>
      </div>
    `;
    lucide.createIcons();
    return;
  }

  container.innerHTML = filtered.map(f => {
    const isSelected = f.hex === state.selectedHex;
    const altColor = getAltitudeColor(f.altitude_ft);
    const airlineBadge = f.airline_iata || f.airline.substring(0, 2).toUpperCase();

    return `
      <div class="flight-card ${isSelected ? 'active-selected' : ''}" data-hex="${f.hex}">
        <div class="flight-card-top">
          <div class="flight-callsign-group">
            <span class="airline-badge">${airlineBadge}</span>
            <span class="flight-callsign">${f.flight}</span>
          </div>
          <span class="flight-altitude-badge" style="background: ${altColor}22; color: ${altColor}; border: 1px solid ${altColor}55;">
            ${f.altitude_ft <= 500 ? 'GND' : `${Math.round(f.altitude_ft).toLocaleString()} FT`}
          </span>
        </div>

        <div class="flight-card-middle">
          <div class="route-text">
            <span>${f.origin_iata || '---'}</span>
            <span class="route-arrow">✈</span>
            <span>${f.dest_iata || '---'}</span>
          </div>
          <span class="aircraft-type-text">${f.aircraft_type}</span>
        </div>

        <div class="flight-card-bottom">
          <div class="flight-status-indicator">
            <span class="status-dot-mini ${f.is_kri ? 'kri' : ''}"></span>
            <span>${f.is_kri ? 'KURDISTAN AIRSPACE' : f.status}</span>
          </div>
          <span>${f.dist_erbil_nm} NM EBL</span>
        </div>
      </div>
    `;
  }).join('');

  // Re-attach card click handlers
  container.querySelectorAll('.flight-card').forEach(card => {
    card.addEventListener('click', () => {
      const hex = card.getAttribute('data-hex');
      selectFlight(hex);
    });
  });
}

// Select Aircraft and Open Drawer
function selectFlight(hex) {
  const flight = state.flights[hex];
  if (!flight) return;

  state.selectedHex = hex;
  playRadarBlip(980, 0.09);

  // Update all markers to reflect selection glow
  Object.entries(state.markers).forEach(([h, marker]) => {
    const f = state.flights[h];
    if (f) marker.setIcon(createPlaneIcon(f, h === hex));
  });

  // Re-render trails with highlighted selection
  Object.keys(state.trails).forEach(h => {
    if (state.flights[h]) renderTrail(h, state.flights[h]);
  });

  // Populate Details
  populateDrawer(flight);

  // Open Drawer
  const drawer = document.getElementById('flight-drawer');
  drawer.classList.add('open');

  // Center Map on Flight
  map.flyTo([flight.lat, flight.lon], Math.max(map.getZoom(), 9), {
    duration: 1.0,
    easeLinearity: 0.25
  });

  // Highlight Card in Sidebar
  document.querySelectorAll('.flight-card').forEach(card => {
    card.classList.toggle('active-selected', card.getAttribute('data-hex') === hex);
  });
}

// Populate Right Telemetry Drawer
function populateDrawer(f) {
  document.getElementById('d-badge').textContent = f.airline_iata || f.flight.substring(0, 2);
  document.getElementById('d-flight').textContent = f.flight;
  document.getElementById('d-airline').textContent = `${f.airline} &bull; ${f.airline_country}`;

  document.getElementById('d-orig-iata').textContent = f.origin_iata || '---';
  document.getElementById('d-orig-city').textContent = f.origin_city || 'Origin';
  document.getElementById('d-dest-iata').textContent = f.dest_iata || '---';
  document.getElementById('d-dest-city').textContent = f.dest_city || 'Destination';

  document.getElementById('d-status').textContent = f.is_kri ? `⚡ ${f.status} (KURDISTAN)` : f.status;
  document.getElementById('d-squawk').textContent = f.squawk || '----';

  // Telemetry Numbers
  document.getElementById('d-alt-ft').textContent = f.altitude_ft.toLocaleString();
  document.getElementById('d-alt-m').textContent = `${f.altitude_m.toLocaleString()} m`;

  const climbEl = document.getElementById('d-climb-rate');
  if (f.climb_rate_fpm > 300) {
    climbEl.textContent = `▲ +${f.climb_rate_fpm} ft/min`;
    climbEl.style.color = 'var(--accent-emerald)';
  } else if (f.climb_rate_fpm < -300) {
    climbEl.textContent = `▼ ${f.climb_rate_fpm} ft/min`;
    climbEl.style.color = 'var(--accent-rose)';
  } else {
    climbEl.textContent = `━ Level Flight`;
    climbEl.style.color = 'var(--accent-cyan)';
  }

  document.getElementById('d-speed-kts').textContent = Math.round(f.speed_kts);
  document.getElementById('d-speed-kmh').textContent = `${f.speed_kmh} km/h`;
  const mach = (f.speed_kts / 661.47).toFixed(2);
  document.getElementById('d-mach').textContent = `Mach ${mach}`;

  document.getElementById('d-heading').textContent = String(Math.round(f.heading)).padStart(3, '0');
  document.getElementById('d-heading-cardinal').textContent = getCardinalDirection(f.heading);
  document.getElementById('d-needle').style.transform = `rotate(${f.heading}deg)`;

  document.getElementById('d-model-name').textContent = f.aircraft_model;
  document.getElementById('d-model-type').textContent = `Type: ${f.aircraft_type}`;
  document.getElementById('d-category').textContent = f.category;

  // Kurdistan Airport Radials
  document.getElementById('d-dist-ebl').textContent = `${f.dist_erbil_nm} NM (${f.dist_erbil_km} km)`;
  document.getElementById('d-radial-ebl').textContent = `Radial ${Math.round(f.radial_erbil)}° to EBL`;
  document.getElementById('d-dist-isu').textContent = `${f.dist_suly_nm} NM (${f.dist_suly_km} km)`;
  document.getElementById('d-radial-isu').textContent = `Radial ${Math.round(f.radial_erbil)}° to ISU`;

  // Tech Specs
  document.getElementById('d-reg').textContent = f.registration || 'N/A';
  document.getElementById('d-hex').textContent = f.hex;
  document.getElementById('d-coords').textContent = `${f.lat.toFixed(4)}° N, ${f.lon.toFixed(4)}° E`;
  document.getElementById('d-sector').textContent = f.is_kri ? 'Kurdistan Region Proper (KRI)' : (f.is_greater_kurdistan ? 'Greater Kurdistan Airspace' : 'Regional Transit Corridor');
  document.getElementById('d-source').textContent = f.source;

  // Update HUD Flight ID if Cockpit Open
  document.getElementById('hud-flight-id').textContent = `${f.flight} (${f.aircraft_model})`;
  document.getElementById('hud-dest').textContent = f.dest_iata || 'EBL';
  document.getElementById('hud-radial').textContent = `${Math.round(f.radial_erbil)}°`;
}

function getCardinalDirection(deg) {
  const dirs = ['N', 'NNE', 'NE', 'ENE', 'E', 'ESE', 'SE', 'SSE', 'S', 'SSW', 'SW', 'WSW', 'W', 'WNW', 'NW', 'NNW'];
  const ix = Math.round(deg / 22.5) % 16;
  return dirs[ix];
}

function closeDrawer() {
  document.getElementById('flight-drawer').classList.remove('open');
  state.selectedHex = null;
  stopFollowing();

  Object.entries(state.markers).forEach(([h, marker]) => {
    const f = state.flights[h];
    if (f) marker.setIcon(createPlaneIcon(f, false));
  });

  document.querySelectorAll('.flight-card').forEach(c => c.classList.remove('active-selected'));
}

// Camera Follow Lock
function startFollowing(hex) {
  state.followingHex = hex;
  const f = state.flights[hex];
  if (!f) return;

  const banner = document.getElementById('follow-banner');
  document.getElementById('follow-callsign').textContent = `${f.flight} (${f.aircraft_model})`;
  banner.style.display = 'block';

  map.flyTo([f.lat, f.lon], 11, { duration: 0.8 });
  playRadarBlip(1100, 0.08);
}

function stopFollowing() {
  state.followingHex = null;
  document.getElementById('follow-banner').style.display = 'none';
}

// Virtual Cockpit Heads-Up Display (HUD) Canvas Renderer
let hudAnimationId = null;
function initCockpitHUD() {
  const modal = document.getElementById('cockpit-modal');
  modal.style.display = 'flex';
  renderHUDFrame();
}

function closeCockpitHUD() {
  document.getElementById('cockpit-modal').style.display = 'none';
  if (hudAnimationId) {
    cancelAnimationFrame(hudAnimationId);
    hudAnimationId = null;
  }
}

function renderHUDFrame() {
  const canvas = document.getElementById('hud-canvas');
  if (!canvas || document.getElementById('cockpit-modal').style.display === 'none') return;
  const ctx = canvas.getContext('2d');
  const w = canvas.width;
  const h = canvas.height;
  const cx = w / 2;
  const cy = h / 2;

  ctx.clearRect(0, 0, w, h);

  const f = state.flights[state.selectedHex] || {
    flight: 'FEG301',
    heading: 116,
    altitude_ft: 37000,
    speed_kts: 460,
    climb_rate_fpm: 0
  };

  // Artificial Horizon Pitch Ladder
  ctx.save();
  ctx.strokeStyle = '#00f2fe';
  ctx.fillStyle = '#00f2fe';
  ctx.lineWidth = 1.8;
  ctx.font = '11px "JetBrains Mono", monospace';

  // Horizon Line
  ctx.beginPath();
  ctx.moveTo(cx - 150, cy);
  ctx.lineTo(cx - 50, cy);
  ctx.moveTo(cx + 50, cy);
  ctx.lineTo(cx + 150, cy);
  ctx.stroke();

  // Aircraft Symbol (Waterline)
  ctx.beginPath();
  ctx.moveTo(cx - 15, cy);
  ctx.lineTo(cx - 5, cy);
  ctx.lineTo(cx, cy + 4);
  ctx.lineTo(cx + 5, cy);
  ctx.lineTo(cx + 15, cy);
  ctx.stroke();

  // Pitch Ladder Lines (+10, +5, -5, -10 degrees)
  const pitchStep = 18;
  [-20, -15, -10, -5, 5, 10, 15, 20].forEach(deg => {
    const y = cy - (deg * 4);
    const lineW = 35;
    ctx.beginPath();
    ctx.moveTo(cx - lineW, y);
    ctx.lineTo(cx - 10, y);
    ctx.moveTo(cx + 10, y);
    ctx.lineTo(cx + lineW, y);
    ctx.stroke();
    ctx.fillText(`${Math.abs(deg)}`, cx - lineW - 18, y + 4);
    ctx.fillText(`${Math.abs(deg)}`, cx + lineW + 6, y + 4);
  });

  // Airspeed Vertical Tape (Left)
  const speedX = 90;
  ctx.strokeRect(speedX - 35, 60, 45, 330);
  ctx.fillText('IAS', speedX - 25, 50);
  ctx.fillStyle = '#fbbf24';
  ctx.font = 'bold 16px "JetBrains Mono", monospace';
  ctx.fillText(`${Math.round(f.speed_kts)}`, speedX - 30, cy + 5);

  // Altimeter Vertical Tape (Right)
  const altX = w - 90;
  ctx.strokeRect(altX - 10, 60, 65, 330);
  ctx.fillStyle = '#00f2fe';
  ctx.font = '11px "JetBrains Mono", monospace';
  ctx.fillText('ALT (FT)', altX - 5, 50);
  ctx.fillStyle = '#10b981';
  ctx.font = 'bold 15px "JetBrains Mono", monospace';
  ctx.fillText(`${Math.round(f.altitude_ft)}`, altX - 5, cy + 5);

  // Compass Heading Ribbon (Top)
  ctx.fillStyle = '#00f2fe';
  ctx.font = 'bold 15px "JetBrains Mono", monospace';
  ctx.fillText(`HDG ${String(Math.round(f.heading)).padStart(3, '0')}°`, cx - 35, 40);

  // Flight Director Crosshair
  ctx.strokeStyle = 'rgba(251, 191, 36, 0.7)';
  ctx.beginPath();
  ctx.arc(cx, cy, 24, 0, Math.PI * 2);
  ctx.stroke();

  ctx.restore();
  hudAnimationId = requestAnimationFrame(renderHUDFrame);
}

// Clocks Update
function updateClocks() {
  const now = new Date();

  // UTC Time
  const utcHours = String(now.getUTCHours()).padStart(2, '0');
  const utcMins = String(now.getUTCMinutes()).padStart(2, '0');
  const utcSecs = String(now.getUTCSeconds()).padStart(2, '0');
  document.getElementById('clock-utc').textContent = `${utcHours}:${utcMins}:${utcSecs}Z`;

  // Kurdistan Local Time (AST is UTC+3)
  const kurdistanDate = new Date(now.getTime() + (3 * 3600 * 1000) + (now.getTimezoneOffset() * 60000));
  const locHours = String(kurdistanDate.getHours()).padStart(2, '0');
  const locMins = String(kurdistanDate.getMinutes()).padStart(2, '0');
  const locSecs = String(kurdistanDate.getSeconds()).padStart(2, '0');
  document.getElementById('clock-local').textContent = `${locHours}:${locMins}:${locSecs}`;
}

// Setup Event Listeners
function setupEvents() {
  // Toggle Sidebar
  document.getElementById('btn-toggle-sidebar').addEventListener('click', () => {
    const sb = document.getElementById('flights-sidebar');
    sb.classList.toggle('collapsed');
  });

  // Recenter Map
  document.getElementById('btn-recenter').addEventListener('click', () => {
    map.flyTo([36.2376, 44.0], 8, { duration: 1.2 });
    playRadarBlip(900, 0.08);
  });

  // Toggle Radar Sweep
  document.getElementById('btn-toggle-radar').addEventListener('click', (e) => {
    state.showRadarSweep = !state.showRadarSweep;
    document.getElementById('radar-sweep').style.display = state.showRadarSweep ? 'block' : 'none';
    e.currentTarget.classList.toggle('active', state.showRadarSweep);
  });

  // Toggle Trails
  document.getElementById('btn-toggle-trails').addEventListener('click', (e) => {
    state.showTrails = !state.showTrails;
    e.currentTarget.classList.toggle('active', state.showTrails);
    if (!state.showTrails) {
      trailLayerGroup.clearLayers();
      state.trails = {};
    } else {
      Object.entries(state.flights).forEach(([h, f]) => renderTrail(h, f));
    }
  });

  // Toggle Borders
  document.getElementById('btn-toggle-borders').addEventListener('click', (e) => {
    state.showBorders = !state.showBorders;
    e.currentTarget.classList.toggle('active', state.showBorders);
    renderBoundaries();
  });

  // Toggle Airports
  document.getElementById('btn-toggle-airports').addEventListener('click', (e) => {
    state.showAirports = !state.showAirports;
    e.currentTarget.classList.toggle('active', state.showAirports);
    renderAirports();
  });

  // Toggle Audio
  document.getElementById('btn-toggle-audio').addEventListener('click', (e) => {
    state.audioEnabled = !state.audioEnabled;
    e.currentTarget.classList.toggle('active', state.audioEnabled);
    if (state.audioEnabled) playRadarBlip(1040, 0.1);
  });

  // Toggle Simulation
  document.getElementById('btn-toggle-sim').addEventListener('click', async () => {
    try {
      const res = await fetch('/api/toggle-simulation', { method: 'POST' });
      const d = await res.json();
      state.simulationMode = d.simulation_mode;
      document.getElementById('btn-toggle-sim').classList.toggle('active', state.simulationMode);
      playRadarBlip(state.simulationMode ? 1200 : 600, 0.12);
      pollFlights();
    } catch (err) {
      console.error(err);
    }
  });

  // Search Filter
  const searchInput = document.getElementById('flight-search');
  const clearSearchBtn = document.getElementById('btn-clear-search');
  searchInput.addEventListener('input', (e) => {
    state.searchQuery = e.target.value.trim();
    clearSearchBtn.style.display = state.searchQuery ? 'block' : 'none';
    renderFlightList();
  });
  clearSearchBtn.addEventListener('click', () => {
    searchInput.value = '';
    state.searchQuery = '';
    clearSearchBtn.style.display = 'none';
    renderFlightList();
  });

  // Filter Chips
  document.querySelectorAll('.filter-chips .chip').forEach(chip => {
    chip.addEventListener('click', () => {
      document.querySelectorAll('.filter-chips .chip').forEach(c => c.classList.remove('active'));
      chip.classList.add('active');
      state.filter = chip.getAttribute('data-filter');
      renderFlightList();
    });
  });

  // Map Style Buttons
  document.querySelectorAll('.map-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.map-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      const style = btn.getAttribute('data-style');
      Object.values(baseTileLayers).forEach(l => map.removeLayer(l));
      if (baseTileLayers[style]) baseTileLayers[style].addTo(map);
    });
  });

  // Drawer Buttons
  document.getElementById('btn-close-drawer').addEventListener('click', closeDrawer);
  document.getElementById('btn-follow-plane').addEventListener('click', () => {
    if (state.selectedHex) startFollowing(state.selectedHex);
  });
  document.getElementById('btn-stop-follow').addEventListener('click', stopFollowing);

  // Cockpit HUD
  document.getElementById('btn-cockpit-hud').addEventListener('click', initCockpitHUD);
  document.getElementById('btn-close-hud').addEventListener('click', closeCockpitHUD);

  // Poll Timer Loop (every 4 seconds)
  setInterval(() => {
    state.nextPollCountdown--;
    if (state.nextPollCountdown <= 0) {
      state.nextPollCountdown = 4;
      pollFlights();
    }
    const cd = document.getElementById('countdown');
    if (cd) cd.textContent = `${state.nextPollCountdown}s`;
  }, 1000);

  // Clocks Loop
  setInterval(updateClocks, 1000);
}

// App Entry Point
window.addEventListener('DOMContentLoaded', async () => {
  lucide.createIcons();
  initMap();
  setupEvents();
  updateClocks();
  await loadBoundaries();
  await loadAirports();
  await pollFlights();
});

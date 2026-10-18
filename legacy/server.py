#!/usr/bin/env python3
"""
Kurdistan SkyRadar - Live Air Traffic Server
Streams and merges live ADS-B & FlightRadar24 flight telemetry over the Kurdistan Region and surrounding airspace.
"""

import os
import sys
import re
import json
import time
import math
import random
import threading
import urllib.request
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler

PORT = 8080
HOST = "0.0.0.0"

# Load Authentic Kurdistan Atlas GeoJSON Boundaries
ATLAS_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "kurdistan_atlas.geojson")
KURDISTAN_ATLAS = {"kri_provinces": {"features": []}, "greater_kurdistan": {"features": []}}
if os.path.exists(ATLAS_PATH):
    try:
        with open(ATLAS_PATH, "r", encoding="utf-8") as f:
            KURDISTAN_ATLAS = json.load(f)
        print(f"Loaded Kurdistan Atlas: {len(KURDISTAN_ATLAS.get('kri_provinces',{}).get('features',[]))} KRI provinces, {len(KURDISTAN_ATLAS.get('greater_kurdistan',{}).get('features',[]))} Greater Kurdistan provinces.")
    except Exception as e:
        print(f"Warning: Failed to load kurdistan_atlas.geojson: {e}")

# Key Kurdish and Regional Airports
AIRPORTS = {
    "EBL": {
        "icao": "ORER", "iata": "EBL", "name": "Erbil International Airport",
        "city": "Erbil (Hewlêr)", "country": "Kurdistan Region, Iraq",
        "lat": 36.2376, "lon": 43.9593, "elevation_ft": 1342,
        "runways": ["18/36 (4,800m)"], "is_kurdistan": True
    },
    "ISU": {
        "icao": "ORSU", "iata": "ISU", "name": "Sulaimaniyah International Airport",
        "city": "Sulaymaniyah (Silêmanî)", "country": "Kurdistan Region, Iraq",
        "lat": 35.5614, "lon": 45.3182, "elevation_ft": 2493,
        "runways": ["13/31 (3,500m)"], "is_kurdistan": True
    },
    "KIK": {
        "icao": "ORKK", "iata": "KIK", "name": "Kirkuk International Airport",
        "city": "Kirkuk", "country": "Iraq",
        "lat": 35.4686, "lon": 44.3489, "elevation_ft": 1083,
        "runways": ["14/32 (2,990m)"], "is_kurdistan": False
    },
    "OSM": {
        "icao": "ORBM", "iata": "OSM", "name": "Mosul International Airport",
        "city": "Mosul", "country": "Iraq",
        "lat": 36.3056, "lon": 43.1492, "elevation_ft": 711,
        "runways": ["14/32 (2,650m)"], "is_kurdistan": False
    },
    "BGW": {
        "icao": "ORBI", "iata": "BGW", "name": "Baghdad International Airport",
        "city": "Baghdad", "country": "Iraq",
        "lat": 33.2625, "lon": 44.2344, "elevation_ft": 114,
        "runways": ["15L/33R (4,000m)", "15R/33L (3,300m)"], "is_kurdistan": False
    },
    "BSR": {
        "icao": "ORMM", "iata": "BSR", "name": "Basra International Airport",
        "city": "Basra", "country": "Iraq",
        "lat": 30.5491, "lon": 47.6622, "elevation_ft": 11,
        "runways": ["14/32 (4,000m)"], "is_kurdistan": False
    },
    "NJF": {
        "icao": "ORNI", "iata": "NJF", "name": "Al Najaf International Airport",
        "city": "Najaf", "country": "Iraq",
        "lat": 31.9897, "lon": 44.4042, "elevation_ft": 105,
        "runways": ["10/28 (3,000m)"], "is_kurdistan": False
    },
    "DIY": {
        "icao": "LTCC", "iata": "DIY", "name": "Diyarbakır Airport",
        "city": "Diyarbakır (Amed)", "country": "Turkey",
        "lat": 37.8939, "lon": 40.2333, "elevation_ft": 2218,
        "runways": ["16/34 (3,549m)"], "is_kurdistan": False
    },
    "VAN": {
        "icao": "LTCI", "iata": "VAN", "name": "Van Ferit Melen Airport",
        "city": "Van (Wan)", "country": "Turkey",
        "lat": 38.4682, "lon": 43.3323, "elevation_ft": 5479,
        "runways": ["03/21 (2,750m)"], "is_kurdistan": False
    },
    "OMH": {
        "icao": "OITR", "iata": "OMH", "name": "Urmia Airport",
        "city": "Urmia (Wrmê)", "country": "Iran",
        "lat": 37.6687, "lon": 45.0694, "elevation_ft": 4344,
        "runways": ["03/21 (3,250m)"], "is_kurdistan": False
    },
    "KAC": {
        "icao": "OSKL", "iata": "KAC", "name": "Qamishli Airport",
        "city": "Qamishli (Qamişlo)", "country": "Syria",
        "lat": 37.0225, "lon": 41.1983, "elevation_ft": 1480,
        "runways": ["03/21 (3,615m)"], "is_kurdistan": False
    },
    "IST": {"icao": "LTFM", "iata": "IST", "name": "Istanbul Airport", "city": "Istanbul", "country": "Turkey", "lat": 41.2753, "lon": 28.7519},
    "SAW": {"icao": "LTFJ", "iata": "SAW", "name": "Sabiha Gökçen Airport", "city": "Istanbul", "country": "Turkey", "lat": 40.8986, "lon": 29.3092},
    "DXB": {"icao": "OMDB", "iata": "DXB", "name": "Dubai International Airport", "city": "Dubai", "country": "UAE", "lat": 25.2532, "lon": 55.3657},
    "DOH": {"icao": "OTHH", "iata": "DOH", "name": "Hamad International Airport", "city": "Doha", "country": "Qatar", "lat": 25.2731, "lon": 51.6081},
    "FRA": {"icao": "EDDF", "iata": "FRA", "name": "Frankfurt Airport", "city": "Frankfurt", "country": "Germany", "lat": 50.0379, "lon": 8.5622},
    "VIE": {"icao": "LOWW", "iata": "VIE", "name": "Vienna International Airport", "city": "Vienna", "country": "Austria", "lat": 48.1103, "lon": 16.5697},
    "DUS": {"icao": "EDDL", "iata": "DUS", "name": "Düsseldorf Airport", "city": "Düsseldorf", "country": "Germany", "lat": 51.2895, "lon": 6.7668},
    "AMM": {"icao": "OJAI", "iata": "AMM", "name": "Queen Alia International", "city": "Amman", "country": "Jordan", "lat": 31.7226, "lon": 35.9932},
    "BEY": {"icao": "OLBA", "iata": "BEY", "name": "Beirut–Rafic Hariri", "city": "Beirut", "country": "Lebanon", "lat": 33.8209, "lon": 35.4884},
    "THR": {"icao": "OIII", "iata": "THR", "name": "Mehrabad International", "city": "Tehran", "country": "Iran", "lat": 35.6892, "lon": 51.3134},
    "IKA": {"icao": "OIIE", "iata": "IKA", "name": "Imam Khomeini International", "city": "Tehran", "country": "Iran", "lat": 35.4161, "lon": 51.1522},
    "MHD": {"icao": "OIMM", "iata": "MHD", "name": "Shahid Hasheminejad", "city": "Mashhad", "country": "Iran", "lat": 36.2352, "lon": 59.6409},
    "TBZ": {"icao": "OITT", "iata": "TBZ", "name": "Tabriz International", "city": "Tabriz", "country": "Iran", "lat": 38.1331, "lon": 46.2353},
    "KWI": {"icao": "OKBK", "iata": "KWI", "name": "Kuwait International", "city": "Kuwait City", "country": "Kuwait", "lat": 29.2267, "lon": 47.9789},
    "AUH": {"icao": "OMAA", "iata": "AUH", "name": "Zayed International", "city": "Abu Dhabi", "country": "UAE", "lat": 24.4330, "lon": 54.6511},
    "SHJ": {"icao": "OMSJ", "iata": "SHJ", "name": "Sharjah International", "city": "Sharjah", "country": "UAE", "lat": 25.3286, "lon": 55.5172},
    "BAH": {"icao": "OBBI", "iata": "BAH", "name": "Bahrain International", "city": "Manama", "country": "Bahrain", "lat": 26.2708, "lon": 50.6336},
    "LHR": {"icao": "EGLL", "iata": "LHR", "name": "Heathrow Airport", "city": "London", "country": "United Kingdom", "lat": 51.4700, "lon": -0.4543},
    "CDG": {"icao": "LFPG", "iata": "CDG", "name": "Charles de Gaulle", "city": "Paris", "country": "France", "lat": 49.0097, "lon": 2.5479},
    "AMS": {"icao": "EHAM", "iata": "AMS", "name": "Schiphol", "city": "Amsterdam", "country": "Netherlands", "lat": 52.3105, "lon": 4.7683},
    "ARN": {"icao": "ESSA", "iata": "ARN", "name": "Stockholm Arlanda", "city": "Stockholm", "country": "Sweden", "lat": 59.6498, "lon": 17.9238},
    "CPH": {"icao": "EKCH", "iata": "CPH", "name": "Copenhagen Airport", "city": "Copenhagen", "country": "Denmark", "lat": 55.6180, "lon": 12.6560},
}

AIRLINES = {
    # Kurdish & Iraqi Operators
    "FEG": {"name": "Fly Erbil", "country": "Kurdistan Region, Iraq", "iata": "HW"},
    "HW": {"name": "Fly Erbil", "country": "Kurdistan Region, Iraq", "iata": "HW"},
    "IAW": {"name": "Iraqi Airways", "country": "Iraq", "iata": "IA"},
    "IA": {"name": "Iraqi Airways", "country": "Iraq", "iata": "IA"},
    "URB": {"name": "UR Airlines", "country": "Iraq", "iata": "UD"},
    "UD": {"name": "UR Airlines", "country": "Iraq", "iata": "UD"},
    "MND": {"name": "Fly Baghdad", "country": "Iraq", "iata": "IF"},
    "IF": {"name": "Fly Baghdad", "country": "Iraq", "iata": "IF"},

    # Regional & Middle East
    "THY": {"name": "Turkish Airlines", "country": "Turkey", "iata": "TK"},
    "TK": {"name": "Turkish Airlines", "country": "Turkey", "iata": "TK"},
    "PGT": {"name": "Pegasus Airlines", "country": "Turkey", "iata": "PC"},
    "PC": {"name": "Pegasus Airlines", "country": "Turkey", "iata": "PC"},
    "TKJ": {"name": "AJet", "country": "Turkey", "iata": "VF"},
    "VF": {"name": "AJet", "country": "Turkey", "iata": "VF"},
    "FDB": {"name": "flydubai", "country": "UAE", "iata": "FZ"},
    "FZ": {"name": "flydubai", "country": "UAE", "iata": "FZ"},
    "QTR": {"name": "Qatar Airways", "country": "Qatar", "iata": "QR"},
    "QR": {"name": "Qatar Airways", "country": "Qatar", "iata": "QR"},
    "UAE": {"name": "Emirates", "country": "UAE", "iata": "EK"},
    "EK": {"name": "Emirates", "country": "UAE", "iata": "EK"},
    "ETD": {"name": "Etihad Airways", "country": "UAE", "iata": "EY"},
    "EY": {"name": "Etihad Airways", "country": "UAE", "iata": "EY"},
    "ABY": {"name": "Air Arabia", "country": "UAE", "iata": "G9"},
    "G9": {"name": "Air Arabia", "country": "UAE", "iata": "G9"},
    "MSR": {"name": "EgyptAir", "country": "Egypt", "iata": "MS"},
    "MS": {"name": "EgyptAir", "country": "Egypt", "iata": "MS"},
    "RJA": {"name": "Royal Jordanian", "country": "Jordan", "iata": "RJ"},
    "RJ": {"name": "Royal Jordanian", "country": "Jordan", "iata": "RJ"},
    "MEA": {"name": "Middle East Airlines", "country": "Lebanon", "iata": "ME"},
    "ME": {"name": "Middle East Airlines", "country": "Lebanon", "iata": "ME"},
    "GFA": {"name": "Gulf Air", "country": "Bahrain", "iata": "GF"},
    "GF": {"name": "Gulf Air", "country": "Bahrain", "iata": "GF"},
    "KAC": {"name": "Kuwait Airways", "country": "Kuwait", "iata": "KU"},
    "KU": {"name": "Kuwait Airways", "country": "Kuwait", "iata": "KU"},
    "JZR": {"name": "Jazeera Airways", "country": "Kuwait", "iata": "J9"},
    "J9": {"name": "Jazeera Airways", "country": "Kuwait", "iata": "J9"},
    "SVA": {"name": "Saudia", "country": "Saudi Arabia", "iata": "SV"},
    "SV": {"name": "Saudia", "country": "Saudi Arabia", "iata": "SV"},
    "OMA": {"name": "Oman Air", "country": "Oman", "iata": "WY"},
    "WY": {"name": "Oman Air", "country": "Oman", "iata": "WY"},

    # Iranian Carriers
    "IRB": {"name": "Iran Air", "country": "Iran", "iata": "IR"},
    "IR": {"name": "Iran Air", "country": "Iran", "iata": "IR"},
    "IRC": {"name": "Iran Aseman", "country": "Iran", "iata": "EP"},
    "EP": {"name": "Iran Aseman", "country": "Iran", "iata": "EP"},
    "TBZ": {"name": "Ata Airlines", "country": "Iran", "iata": "I3"},
    "I3": {"name": "Ata Airlines", "country": "Iran", "iata": "I3"},
    "VRH": {"name": "Varesh Airlines", "country": "Iran", "iata": "VR"},
    "VR": {"name": "Varesh Airlines", "country": "Iran", "iata": "VR"},
    "MHD": {"name": "Zagros Airlines", "country": "Iran", "iata": "ZV"},
    "ZV": {"name": "Zagros Airlines", "country": "Iran", "iata": "ZV"},
    "IRZ": {"name": "Saha Airlines", "country": "Iran", "iata": "IRZ"},
    "B9": {"name": "Iran Air Tour", "country": "Iran", "iata": "B9"},

    # International & European
    "AIC": {"name": "Air India", "country": "India", "iata": "AI"},
    "AI": {"name": "Air India", "country": "India", "iata": "AI"},
    "IGO": {"name": "IndiGo", "country": "India", "iata": "6E"},
    "6E": {"name": "IndiGo", "country": "India", "iata": "6E"},
    "DLH": {"name": "Lufthansa", "country": "Germany", "iata": "LH"},
    "LH": {"name": "Lufthansa", "country": "Germany", "iata": "LH"},
    "AUA": {"name": "Austrian Airlines", "country": "Austria", "iata": "OS"},
    "OS": {"name": "Austrian Airlines", "country": "Austria", "iata": "OS"},
    "CFG": {"name": "Condor", "country": "Germany", "iata": "DE"},
    "DE": {"name": "Condor", "country": "Germany", "iata": "DE"},
    "WZZ": {"name": "Wizz Air", "country": "Hungary", "iata": "W6"},
    "W6": {"name": "Wizz Air", "country": "Hungary", "iata": "W6"},
    "BAW": {"name": "British Airways", "country": "United Kingdom", "iata": "BA"},
    "BA": {"name": "British Airways", "country": "United Kingdom", "iata": "BA"},
    "AFR": {"name": "Air France", "country": "France", "iata": "AF"},
    "AF": {"name": "Air France", "country": "France", "iata": "AF"},
    "KLM": {"name": "KLM", "country": "Netherlands", "iata": "KL"},
    "KL": {"name": "KLM", "country": "Netherlands", "iata": "KL"},
    "SEH": {"name": "Sky Express", "country": "Greece", "iata": "GQ"},
    "GQ": {"name": "Sky Express", "country": "Greece", "iata": "GQ"},
    "FIA": {"name": "Fly Lili", "country": "Romania", "iata": "FL"},
    "FL": {"name": "Fly Lili", "country": "Romania", "iata": "FL"},
    "RCH": {"name": "US Air Force (AMC)", "country": "United States", "iata": "MIL"},
    "ASY": {"name": "Royal Australian Air Force", "country": "Australia", "iata": "MIL"},
}

AIRCRAFT_MODELS = {
    "A319": {"name": "Airbus A319-100", "category": "Jet Airliner", "widebody": False},
    "A320": {"name": "Airbus A320-200", "category": "Jet Airliner", "widebody": False},
    "A20N": {"name": "Airbus A320neo", "category": "Jet Airliner", "widebody": False},
    "A321": {"name": "Airbus A321-200", "category": "Jet Airliner", "widebody": False},
    "A21N": {"name": "Airbus A321neo", "category": "Jet Airliner", "widebody": False},
    "A332": {"name": "Airbus A330-200", "category": "Widebody Jet", "widebody": True},
    "A333": {"name": "Airbus A330-300", "category": "Widebody Jet", "widebody": True},
    "A339": {"name": "Airbus A330-900neo", "category": "Widebody Jet", "widebody": True},
    "A359": {"name": "Airbus A350-900", "category": "Widebody Jet", "widebody": True},
    "A35K": {"name": "Airbus A350-1000", "category": "Widebody Jet", "widebody": True},
    "A388": {"name": "Airbus A380-800", "category": "Super Jumbo", "widebody": True},
    "A310": {"name": "Airbus A310-300", "category": "Widebody Jet", "widebody": True},
    "B737": {"name": "Boeing 737-700", "category": "Jet Airliner", "widebody": False},
    "B738": {"name": "Boeing 737-800", "category": "Jet Airliner", "widebody": False},
    "B739": {"name": "Boeing 737-900ER", "category": "Jet Airliner", "widebody": False},
    "B38M": {"name": "Boeing 737 MAX 8", "category": "Jet Airliner", "widebody": False},
    "B39M": {"name": "Boeing 737 MAX 9", "category": "Jet Airliner", "widebody": False},
    "B772": {"name": "Boeing 777-200ER", "category": "Widebody Jet", "widebody": True},
    "B77W": {"name": "Boeing 777-300ER", "category": "Widebody Jet", "widebody": True},
    "B788": {"name": "Boeing 787-8 Dreamliner", "category": "Widebody Jet", "widebody": True},
    "B789": {"name": "Boeing 787-9 Dreamliner", "category": "Widebody Jet", "widebody": True},
    "B78X": {"name": "Boeing 787-10 Dreamliner", "category": "Widebody Jet", "widebody": True},
    "B744": {"name": "Boeing 747-400 Jumbo", "category": "Jumbo Jet", "widebody": True},
    "B748": {"name": "Boeing 747-8 Intercontinental", "category": "Jumbo Jet", "widebody": True},
    "E190": {"name": "Embraer E190", "category": "Regional Jet", "widebody": False},
    "E195": {"name": "Embraer E195", "category": "Regional Jet", "widebody": False},
    "CRJ9": {"name": "Bombardier CRJ-900", "category": "Regional Jet", "widebody": False},
    "AT76": {"name": "ATR 72-600", "category": "Turboprop", "widebody": False},
    "C172": {"name": "Cessna 172 Skyhawk", "category": "Light Aircraft", "widebody": False},
    "C17": {"name": "Boeing C-17 Globemaster III", "category": "Military Transport", "widebody": True},
    "A400": {"name": "Airbus A400M Atlas", "category": "Military Transport", "widebody": True},
    "C130": {"name": "Lockheed C-130 Hercules", "category": "Military Transport", "widebody": False},
    "K35R": {"name": "Boeing KC-135 Stratotanker", "category": "Military Refueler", "widebody": True},
}

def point_in_polygon(lon, lat, poly):
    """Ray casting point in polygon test."""
    n = len(poly)
    inside = False
    p1x, p1y = poly[0]
    for i in range(n + 1):
        p2x, p2y = poly[i % n]
        if lat > min(p1y, p2y) and lat <= max(p1y, p2y) and lon <= max(p1x, p2x):
            if p1y != p2y:
                xinters = (lat - p1y) * (p2x - p1x) / (p2y - p1y) + p1x
            if p1x == p2x or lon <= xinters:
                inside = not inside
        p1x, p1y = p2x, p2y
    return inside

def point_in_feature(lon, lat, feature):
    """Check if point is inside a GeoJSON Polygon or MultiPolygon feature."""
    geom = feature.get("geometry", {})
    gt = geom.get("type")
    coords = geom.get("coordinates", [])
    if gt == "Polygon":
        if coords and point_in_polygon(lon, lat, coords[0]):
            return True
    elif gt == "MultiPolygon":
        for poly in coords:
            if poly and point_in_polygon(lon, lat, poly[0]):
                return True
    return False

def is_in_kri(lon, lat):
    """True if coordinates are inside the official Kurdistan Region of Iraq."""
    kri_feats = KURDISTAN_ATLAS.get("kri_provinces", {}).get("features", [])
    return any(point_in_feature(lon, lat, f) for f in kri_feats)

def is_in_greater_kurdistan(lon, lat):
    """True if coordinates are inside any of the 4 parts of Greater Kurdistan."""
    greater_feats = KURDISTAN_ATLAS.get("greater_kurdistan", {}).get("features", [])
    return any(point_in_feature(lon, lat, f) for f in greater_feats)

def haversine(lat1, lon1, lat2, lon2):
    """Calculates distance in km and nm between two points."""
    R_KM = 6371.0
    R_NM = 3440.065
    dlat = math.radians(lat2 - lat1)
    dlon = math.radians(lon2 - lon1)
    a = (math.sin(dlat / 2) ** 2 +
         math.cos(math.radians(lat1)) * math.cos(math.radians(lat2)) *
         math.sin(dlon / 2) ** 2)
    c = 2 * math.atan2(math.sqrt(a), math.sqrt(1 - a))
    return {
        "km": round(R_KM * c, 1),
        "nm": round(R_NM * c, 1)
    }

def calculate_bearing(lat1, lon1, lat2, lon2):
    """Bearing in degrees from point 1 to point 2."""
    lat1_r = math.radians(lat1)
    lat2_r = math.radians(lat2)
    dlon_r = math.radians(lon2 - lon1)
    y = math.sin(dlon_r) * math.cos(lat2_r)
    x = (math.cos(lat1_r) * math.sin(lat2_r) -
         math.sin(lat1_r) * math.cos(lat2_r) * math.cos(dlon_r))
    bearing = (math.degrees(math.atan2(y, x)) + 360) % 360
    return round(bearing, 1)

class FlightDataService:
    def __init__(self):
        self.lock = threading.Lock()
        self.flights = {}
        self.flight_trails = {}  # hex -> list of [lat, lon, alt, timestamp]
        self.simulation_mode = False
        self.last_fetch_time = 0
        self.fetch_error = None
        self.source_status = {"fr24": False, "adsb": False}
        self.init_simulation_flights()

    def init_simulation_flights(self):
        """Curated realistic flights over Kurdistan for demonstration/night testing."""
        self.sim_templates = [
            {
                "hex": "7048A1", "flight": "FEG301", "callsign": "FEG301",
                "airline_code": "FEG", "model_code": "B738", "reg": "YI-BAH",
                "origin": "DUS", "dest": "EBL",
                "start_lat": 37.45, "start_lon": 42.10, "end_lat": 36.2376, "end_lon": 43.9593,
                "start_alt": 28000, "end_alt": 2000, "speed": 340, "progress": 0.45
            },
            {
                "hex": "4BBD06", "flight": "TKJ1XN", "callsign": "VF131",
                "airline_code": "TKJ", "model_code": "B38M", "reg": "TC-OHF",
                "origin": "SAW", "dest": "BGW",
                "start_lat": 38.10, "start_lon": 42.20, "end_lat": 33.2625, "end_lon": 44.2344,
                "start_alt": 37000, "end_alt": 37000, "speed": 460, "progress": 0.35
            },
            {
                "hex": "73348C", "flight": "IRB9734", "callsign": "B99734",
                "airline_code": "IRB", "model_code": "A310", "reg": "EP-MDL",
                "origin": "IST", "dest": "MHD",
                "start_lat": 38.20, "start_lon": 43.00, "end_lat": 36.50, "end_lon": 47.80,
                "start_alt": 39000, "end_alt": 39000, "speed": 485, "progress": 0.28
            },
            {
                "hex": "4BC721", "flight": "PGT408", "callsign": "PGT408",
                "airline_code": "PGT", "model_code": "A21N", "reg": "TC-RDE",
                "origin": "SAW", "dest": "ISU",
                "start_lat": 37.60, "start_lon": 42.80, "end_lat": 35.5614, "end_lon": 45.3182,
                "start_alt": 31000, "end_alt": 3200, "speed": 380, "progress": 0.55
            },
            {
                "hex": "06A144", "flight": "QTR4PG", "callsign": "QR492",
                "airline_code": "QTR", "model_code": "A359", "reg": "A7-AMD",
                "origin": "DOH", "dest": "LHR",
                "start_lat": 34.00, "start_lon": 45.80, "end_lat": 38.50, "end_lon": 41.50,
                "start_alt": 40000, "end_alt": 40000, "speed": 510, "progress": 0.30
            },
            {
                "hex": "896422", "flight": "UAE71A", "callsign": "EK71",
                "airline_code": "UAE", "model_code": "B77W", "reg": "A6-EQB",
                "origin": "DXB", "dest": "CDG",
                "start_lat": 33.80, "start_lon": 45.20, "end_lat": 38.20, "end_lon": 41.80,
                "start_alt": 36000, "end_alt": 36000, "speed": 495, "progress": 0.20
            },
            {
                "hex": "72015B", "flight": "IAW202", "callsign": "IAW202",
                "airline_code": "IAW", "model_code": "A220", "reg": "YI-ARI",
                "origin": "BGW", "dest": "EBL",
                "start_lat": 33.50, "start_lon": 44.20, "end_lat": 36.2376, "end_lon": 43.9593,
                "start_alt": 4000, "end_alt": 2200, "speed": 320, "progress": 0.70
            },
            {
                "hex": "3C6544", "flight": "DLH6ER", "callsign": "LH602",
                "airline_code": "DLH", "model_code": "A333", "reg": "D-AIKO",
                "origin": "FRA", "dest": "EBL",
                "start_lat": 37.80, "start_lon": 42.50, "end_lat": 36.2376, "end_lon": 43.9593,
                "start_alt": 24000, "end_alt": 2500, "speed": 350, "progress": 0.60
            },
            {
                "hex": "896A11", "flight": "FDB803", "callsign": "FZ803",
                "airline_code": "FDB", "model_code": "B38M", "reg": "A6-FMK",
                "origin": "DXB", "dest": "ISU",
                "start_lat": 33.50, "start_lon": 45.50, "end_lat": 35.5614, "end_lon": 45.3182,
                "start_alt": 18000, "end_alt": 3000, "speed": 330, "progress": 0.65
            },
            {
                "hex": "4BA123", "flight": "THY771", "callsign": "TK771",
                "airline_code": "THY", "model_code": "A321", "reg": "TC-JTL",
                "origin": "IST", "dest": "EBL",
                "start_lat": 37.50, "start_lon": 43.10, "end_lat": 36.2376, "end_lon": 43.9593,
                "start_alt": 19000, "end_alt": 2400, "speed": 310, "progress": 0.80
            }
        ]
        self.sim_state = []
        for t in self.sim_templates:
            self.sim_state.append(dict(t))

    def fetch_fr24(self):
        """Fetch flights from FlightRadar24 regional zone."""
        # Regional box covering Kurdistan, Iraq, eastern Syria, southern Turkey, western Iran
        url = "https://data-cloud.flightradar24.com/zones/fcgi/feed.js?bounds=40.5,32.5,37.0,48.0"
        headers = {
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Accept": "application/json"
        }
        req = urllib.request.Request(url, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=5) as response:
                data = json.loads(response.read().decode("utf-8"))
                self.source_status["fr24"] = True
                return data
        except Exception as e:
            self.source_status["fr24"] = False
            return {}

    def fetch_adsb(self):
        """Fetch high-precision ADS-B telemetry from adsb.lol / adsb.fi."""
        endpoints = [
            "https://api.adsb.lol/v2/point/36.2/44.0/350",
            "https://opendata.adsb.fi/api/v2/lat/36.2/lon/44.0/dist/350"
        ]
        headers = {
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) SkyRadar/1.0",
            "Accept": "application/json"
        }
        for ep in endpoints:
            try:
                req = urllib.request.Request(ep, headers=headers)
                with urllib.request.urlopen(req, timeout=5) as response:
                    data = json.loads(response.read().decode("utf-8"))
                    aircraft = data.get("ac", data.get("aircraft", []))
                    if aircraft:
                        self.source_status["adsb"] = True
                        return aircraft
            except Exception:
                continue
        self.source_status["adsb"] = False
        return []

    def update_simulated_traffic(self, dt=3.0):
        """Smoothly advance simulated flights for live demo."""
        sim_flights = {}
        now = time.time()
        for s in self.sim_state:
            # Advance progress
            speed_knots = s.get("speed", 400)
            # Roughly 1 nm per ~7 seconds at 500kt
            distance_ratio_step = (speed_knots * 0.514444 * dt) / 500000.0
            s["progress"] = (s.get("progress", 0.0) + distance_ratio_step)
            if s["progress"] > 1.0:
                s["progress"] = 0.05  # Loop back or reverse

            prog = s["progress"]
            # Interpolate coordinates
            lat = s["start_lat"] + (s["end_lat"] - s["start_lat"]) * prog
            lon = s["start_lon"] + (s["end_lon"] - s["start_lon"]) * prog

            # Altitude curve
            alt = int(s["start_alt"] + (s["end_alt"] - s["start_alt"]) * prog)
            heading = calculate_bearing(s["start_lat"], s["start_lon"], s["end_lat"], s["end_lon"])

            hex_code = s["hex"]
            sim_flights[hex_code] = self.enrich_flight({
                "hex": hex_code,
                "flight": s["flight"],
                "callsign": s["callsign"],
                "lat": round(lat, 5),
                "lon": round(lon, 5),
                "track": heading,
                "alt_baro": alt,
                "gs": speed_knots,
                "squawk": "2412",
                "t": s["model_code"],
                "r": s["reg"],
                "origin": s["origin"],
                "dest": s["dest"],
                "source": "SIMULATION"
            })
        return sim_flights

    def enrich_flight(self, raw):
        """Standardize and enrich flight telemetry with airports, airlines, and Kurdish airspace logic."""
        hex_code = raw.get("hex", "").strip().upper()
        flight_callsign = (raw.get("flight") or raw.get("callsign") or "").strip().upper()
        if not flight_callsign and hex_code:
            flight_callsign = f"ICAO-{hex_code}"

        lat = float(raw.get("lat", 0.0))
        lon = float(raw.get("lon", 0.0))
        alt_baro = raw.get("alt_baro", raw.get("altitude", 0))
        if isinstance(alt_baro, str):
            alt_ft = 0 if alt_baro.lower() == "ground" else 0
        else:
            alt_ft = int(alt_baro or 0)
        alt_m = int(alt_ft * 0.3048)

        speed_kts = float(raw.get("gs", raw.get("speed", 0.0)) or 0.0)
        speed_kmh = int(speed_kts * 1.852)

        track_deg = float(raw.get("track", raw.get("heading", 0.0)) or 0.0)
        climb_rate = int(raw.get("baro_rate", raw.get("vertical_speed", 0)) or 0)

        # Aircraft model
        type_code = (raw.get("t") or raw.get("model") or "").strip().upper()
        model_info = AIRCRAFT_MODELS.get(type_code, {
            "name": type_code if type_code else "Standard Aircraft",
            "category": "Commercial Jet",
            "widebody": False
        })

        # Airline identification
        airline_info = None
        # Try 3-letter ICAO, then 2-letter IATA from callsign
        m = re.match(r"^([A-Za-z]{2,3})", flight_callsign)
        if m:
            prefix = m.group(1).upper()
            if prefix in AIRLINES:
                airline_info = AIRLINES[prefix]
            elif len(prefix) == 3 and prefix[:2] in AIRLINES:
                airline_info = AIRLINES[prefix[:2]]
        if not airline_info and len(flight_callsign) >= 2:
            p2 = flight_callsign[:2].upper()
            if p2 in AIRLINES:
                airline_info = AIRLINES[p2]

        if not airline_info:
            airline_info = {
                "name": f"Flight {flight_callsign}" if flight_callsign else "Commercial Flight",
                "country": "International",
                "iata": flight_callsign[:2] if len(flight_callsign) >= 2 else "AIR"
            }

        # Origin / Destination
        orig_code = (raw.get("origin") or "").strip().upper()
        dest_code = (raw.get("dest") or raw.get("destination") or "").strip().upper()

        origin_airport = AIRPORTS.get(orig_code, {
            "name": f"Airport ({orig_code})" if orig_code else "Departed",
            "city": orig_code or "Unknown Origin",
            "iata": orig_code
        })
        dest_airport = AIRPORTS.get(dest_code, {
            "name": f"Airport ({dest_code})" if dest_code else "En Route",
            "city": dest_code or "Unknown Destination",
            "iata": dest_code
        })

        # Distance to Erbil and Sulaymaniyah
        dist_ebl = haversine(lat, lon, AIRPORTS["EBL"]["lat"], AIRPORTS["EBL"]["lon"])
        dist_isu = haversine(lat, lon, AIRPORTS["ISU"]["lat"], AIRPORTS["ISU"]["lon"])
        radial_ebl = calculate_bearing(AIRPORTS["EBL"]["lat"], AIRPORTS["EBL"]["lon"], lat, lon)

        # Kurdish Airspace Geofence Analysis (Authentic Kurdistan Atlas)
        is_kri = is_in_kri(lon, lat)
        is_greater = is_in_greater_kurdistan(lon, lat)

        # Tactical Status Determination
        status = "Cruising Airspace"
        if alt_ft <= 500:
            status = "On Ground"
        elif dest_code in ("EBL", "ORER") and dist_ebl["nm"] <= 40:
            status = f"Approach Erbil (EBL {dist_ebl['nm']} NM)"
        elif dest_code in ("ISU", "ORSU") and dist_isu["nm"] <= 40:
            status = f"Approach Sulaymaniyah (ISU {dist_isu['nm']} NM)"
        elif orig_code in ("EBL", "ORER") and dist_ebl["nm"] <= 40:
            status = "Departing Erbil (EBL)"
        elif orig_code in ("ISU", "ORSU") and dist_isu["nm"] <= 40:
            status = "Departing Sulaymaniyah (ISU)"
        elif climb_rate > 350:
            status = f"Climbing (FL{alt_ft//100})"
        elif climb_rate < -350:
            status = f"Descending (FL{alt_ft//100})"
        else:
            status = f"Level Flight (FL{alt_ft//100})"

        # Record trail breadcrumbs
        now = time.time()
        with self.lock:
            if hex_code not in self.flight_trails:
                self.flight_trails[hex_code] = []
            trail = self.flight_trails[hex_code]
            # Add point if moved significantly or empty
            if not trail or abs(trail[-1][0] - lat) > 0.002 or abs(trail[-1][1] - lon) > 0.002:
                trail.append([round(lat, 4), round(lon, 4), alt_ft, int(now)])
                if len(trail) > 30:
                    self.flight_trails[hex_code] = trail[-30:]

        return {
            "hex": hex_code,
            "flight": flight_callsign,
            "airline": airline_info["name"],
            "airline_country": airline_info.get("country", ""),
            "airline_iata": airline_info.get("iata", ""),
            "aircraft_model": model_info["name"],
            "aircraft_type": type_code or "AIRCRAFT",
            "category": model_info["category"],
            "widebody": model_info.get("widebody", False),
            "registration": raw.get("r", raw.get("reg", "N/A")),
            "lat": lat,
            "lon": lon,
            "altitude_ft": alt_ft,
            "altitude_m": alt_m,
            "speed_kts": round(speed_kts, 1),
            "speed_kmh": speed_kmh,
            "heading": round(track_deg, 1),
            "climb_rate_fpm": climb_rate,
            "squawk": raw.get("squawk", "---"),
            "origin_iata": orig_code,
            "origin_city": origin_airport.get("city", orig_code),
            "origin_name": origin_airport.get("name", ""),
            "dest_iata": dest_code,
            "dest_city": dest_airport.get("city", dest_code),
            "dest_name": dest_airport.get("name", ""),
            "dist_erbil_km": dist_ebl["km"],
            "dist_erbil_nm": dist_ebl["nm"],
            "dist_suly_km": dist_isu["km"],
            "dist_suly_nm": dist_isu["nm"],
            "radial_erbil": radial_ebl,
            "is_kri": is_kri,
            "is_greater_kurdistan": is_greater,
            "status": status,
            "trail": self.flight_trails.get(hex_code, []),
            "source": raw.get("source", "ADS-B LIVE"),
            "last_seen": int(now)
        }

    def refresh(self):
        """Fetch and merge real-time flight data."""
        now = time.time()
        merged = {}

        # 1. Fetch FlightRadar24
        fr24_raw = self.fetch_fr24()
        for k, v in fr24_raw.items():
            if isinstance(v, list) and len(v) >= 14:
                try:
                    hex_code = str(v[0]).upper()
                    lat = float(v[1])
                    lon = float(v[2])
                    track = float(v[3])
                    alt = int(v[4])
                    speed = float(v[5])
                    squawk = str(v[6])
                    model = str(v[8])
                    reg = str(v[9])
                    orig = str(v[11]) if v[11] else ""
                    dest = str(v[12]) if v[12] else ""
                    flight_no = str(v[13]) if v[13] else (str(v[16]) if len(v) > 16 and v[16] else hex_code)
                    climb = int(v[15]) if len(v) > 15 and isinstance(v[15], (int, float)) else 0

                    merged[hex_code] = {
                        "hex": hex_code,
                        "flight": flight_no,
                        "lat": lat,
                        "lon": lon,
                        "track": track,
                        "alt_baro": alt,
                        "gs": speed,
                        "squawk": squawk,
                        "t": model,
                        "r": reg,
                        "origin": orig,
                        "dest": dest,
                        "baro_rate": climb,
                        "source": "FR24"
                    }
                except Exception:
                    continue

        # 2. Fetch ADS-B (adsb.lol / adsb.fi) to enrich or augment
        adsb_raw = self.fetch_adsb()
        for a in adsb_raw:
            try:
                hex_code = str(a.get("hex", "")).upper()
                if not hex_code:
                    continue
                lat = a.get("lat")
                lon = a.get("lon")
                if lat is None or lon is None:
                    continue

                if hex_code in merged:
                    # Enrich with higher fidelity ADS-B telemetry
                    m = merged[hex_code]
                    if a.get("gs") is not None:
                        m["gs"] = a.get("gs")
                    if a.get("alt_baro") is not None and a.get("alt_baro") != "ground":
                        m["alt_baro"] = a.get("alt_baro")
                    if a.get("track") is not None:
                        m["track"] = a.get("track")
                    if a.get("baro_rate") is not None:
                        m["baro_rate"] = a.get("baro_rate")
                    if not m.get("t") and a.get("t"):
                        m["t"] = a.get("t")
                    if not m.get("r") and a.get("r"):
                        m["r"] = a.get("r")
                else:
                    # New aircraft discovered via ADS-B
                    merged[hex_code] = {
                        "hex": hex_code,
                        "flight": (a.get("flight") or hex_code).strip(),
                        "lat": float(lat),
                        "lon": float(lon),
                        "track": float(a.get("track", 0.0) or 0.0),
                        "alt_baro": a.get("alt_baro", 0),
                        "gs": float(a.get("gs", 0.0) or 0.0),
                        "squawk": str(a.get("squawk", "")),
                        "t": a.get("t", ""),
                        "r": a.get("r", ""),
                        "baro_rate": a.get("baro_rate", 0),
                        "source": "ADS-B"
                    }
            except Exception:
                continue

        # Process and enrich all flights
        enriched = {}
        for h, raw in merged.items():
            try:
                ef = self.enrich_flight(raw)
                enriched[h] = ef
            except Exception:
                continue

        # If simulation mode is active OR if live regional flights are sparse (e.g. late night)
        if self.simulation_mode or len(enriched) < 3:
            sim_flights = self.update_simulated_traffic()
            for sh, sf in sim_flights.items():
                if sh not in enriched:
                    enriched[sh] = sf

        with self.lock:
            self.flights = enriched
            self.last_fetch_time = now

            # Clean stale trails
            stale_threshold = now - 1800  # 30 mins
            active_hexes = set(self.flights.keys())
            stale = [h for h in self.flight_trails if h not in active_hexes]
            for h in stale:
                if self.flight_trails[h] and self.flight_trails[h][-1][3] < stale_threshold:
                    del self.flight_trails[h]

    def get_snapshot(self):
        """Returns JSON snapshot of all flights and statistics."""
        with self.lock:
            flight_list = list(self.flights.values())
            now = time.time()

            kri_count = sum(1 for f in flight_list if f["is_kri"])
            greater_count = sum(1 for f in flight_list if f["is_greater_kurdistan"])
            ebl_traffic = sum(1 for f in flight_list if f["dest_iata"] in ("EBL", "ORER") or f["origin_iata"] in ("EBL", "ORER"))
            isu_traffic = sum(1 for f in flight_list if f["dest_iata"] in ("ISU", "ORSU") or f["origin_iata"] in ("ISU", "ORSU"))

            return {
                "timestamp": int(now),
                "total_flights": len(flight_list),
                "kri_airspace_flights": kri_count,
                "greater_kurdistan_flights": greater_count,
                "erbil_flights": ebl_traffic,
                "sulaymaniyah_flights": isu_traffic,
                "simulation_mode": self.simulation_mode,
                "sources": self.source_status,
                "flights": flight_list
            }

# Global Data Service Singleton
data_service = FlightDataService()

def background_poller():
    """Background worker continuously refreshing flight telemetry."""
    while True:
        try:
            data_service.refresh()
        except Exception as e:
            data_service.fetch_error = str(e)
        time.sleep(4.0)

class RadarRequestHandler(SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/api/flights" or self.path.startswith("/api/flights?"):
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
            self.end_headers()
            data = data_service.get_snapshot()
            self.wfile.write(json.dumps(data).encode("utf-8"))

        elif self.path == "/api/airports":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps(AIRPORTS).encode("utf-8"))

        elif self.path == "/api/boundaries":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps(KURDISTAN_ATLAS).encode("utf-8"))

        elif self.path == "/api/toggle-simulation":
            data_service.simulation_mode = not data_service.simulation_mode
            data_service.refresh()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps({"simulation_mode": data_service.simulation_mode}).encode("utf-8"))

        else:
            # Serve static files from current directory
            return super().do_GET()

    def do_POST(self):
        if self.path == "/api/toggle-simulation":
            data_service.simulation_mode = not data_service.simulation_mode
            data_service.refresh()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps({"simulation_mode": data_service.simulation_mode}).encode("utf-8"))
        else:
            self.send_error(404, "Endpoint not found")

def run_server():
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    # Initial data pull
    print("Initiating Kurdistan SkyRadar telemetry engine...")
    data_service.refresh()
    print(f"Initial flight count: {len(data_service.flights)}")

    # Start background polling thread
    poller = threading.Thread(target=background_poller, daemon=True)
    poller.start()

    server = ThreadingHTTPServer((HOST, PORT), RadarRequestHandler)
    print(f"============================================================")
    print(f"   KURDISTAN SKYRADAR SERVER ACTIVE ON http://localhost:{PORT}")
    print(f"============================================================")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down SkyRadar server...")
        server.server_close()

if __name__ == "__main__":
    run_server()

//! Static aviation reference data: airports across all four parts of Kurdistan,
//! regional hubs, airlines, aircraft types, and military/government classifiers.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Airport {
    pub iata: &'static str,
    pub icao: &'static str,
    pub name: &'static str,
    pub city: &'static str,
    pub country: &'static str,
    pub lat: f64,
    pub lon: f64,
    /// Airport sits inside Greater Kurdistan.
    pub kurdish: bool,
    /// Primary KRI international hub (`EBL`, `ISU`, `DHK`, `KIK`).
    pub kri_hub: bool,
}

macro_rules! apt {
    ($iata:literal, $icao:literal, $name:literal, $city:literal, $country:literal, $lat:literal, $lon:literal, $k:literal, $hub:literal) => {
        Airport {
            iata: $iata,
            icao: $icao,
            name: $name,
            city: $city,
            country: $country,
            lat: $lat,
            lon: $lon,
            kurdish: $k,
            kri_hub: $hub,
        }
    };
}

pub static AIRPORTS: &[Airport] = &[
    // ── Başûr (Southern Kurdistan / KRI & Disputed) ─────────────────────────
    apt!("EBL", "ORER", "Hewlêr (Erbil) International Airport", "Hewlêr", "Başûr · KRI", 36.2376, 43.9593, true, true),
    apt!("ISU", "ORSU", "Silêmanî International Airport", "Silêmanî", "Başûr · KRI", 35.5614, 45.3182, true, true),
    apt!("KIK", "ORKK", "Kerkûk International Airport", "Kerkûk", "Başûr", 35.4686, 44.3489, true, true),
    apt!("DHK", "ORDA", "Duhok International Airport", "Duhok", "Başûr · KRI", 36.8580, 42.8750, true, true),
    apt!("HRR", "ORHB", "Herîr Air Base", "Herîr (Hewlêr)", "Başûr · KRI", 36.5350, 44.3430, true, false),

    // ── Rojava (Western Kurdistan) ──────────────────────────────────────────
    apt!("KAC", "OSKL", "Qamişlo International Airport", "Qamişlo", "Rojava", 37.0206, 41.1914, true, false),
    apt!("RML", "OSRM", "Rimêlan Airfield", "Dêrik / Rimêlan", "Rojava", 36.9950, 42.0250, true, false),

    // ── Bakur (Northern Kurdistan) ──────────────────────────────────────────
    apt!("DIY", "LTCC", "Amed (Diyarbakır) Airport", "Amed", "Bakur", 37.8939, 40.2010, true, false),
    apt!("VAN", "LTCI", "Wan Ferit Melen Airport", "Wan", "Bakur", 38.4682, 43.3323, true, false),
    apt!("BAL", "LTCJ", "Êlih (Batman) Airport", "Êlih", "Bakur", 37.9290, 41.1166, true, false),
    apt!("MQM", "LTCR", "Mêrdîn Airport", "Mêrdîn", "Bakur", 37.2233, 40.6317, true, false),
    apt!("NKT", "LTCV", "Şirnex Şerafettin Elçi Airport", "Cizîr / Şirnex", "Bakur", 37.3647, 42.0582, true, false),
    apt!("YKO", "LTCW", "Gever (Yüksekova) Airport", "Gever / Colemêrg", "Bakur", 37.5497, 44.2381, true, false),
    apt!("GNY", "LTCS", "Riha (Şanlıurfa) GAP Airport", "Riha", "Bakur", 37.4457, 38.8956, true, false),
    apt!("GZT", "LTAJ", "Dîlok (Gaziantep) Airport", "Dîlok", "Bakur", 36.9472, 37.4787, true, false),
    apt!("EZS", "LTCA", "Xarpêt (Elazığ) Airport", "Xarpêt", "Bakur", 38.6069, 39.2914, true, false),
    apt!("ERZ", "LTCE", "Erzirom (Erzurum) Airport", "Erzirom", "Bakur", 39.9565, 41.1702, true, false),
    apt!("ERC", "LTCD", "Erzingan (Erzincan) Airport", "Erzingan", "Bakur", 39.7102, 39.5270, true, false),
    apt!("MLX", "LTAT", "Meletî (Malatya) Airport", "Meletî", "Bakur", 38.4353, 38.0910, true, false),
    apt!("ADF", "LTCP", "Semsûr (Adıyaman) Airport", "Semsûr", "Bakur", 37.7314, 38.4689, true, false),
    apt!("MSR", "LTCK", "Mûş Airport", "Mûş", "Bakur", 38.7478, 41.6612, true, false),
    apt!("BGG", "LTCU", "Çewlîg (Bingöl) Airport", "Çewlîg", "Bakur", 38.8611, 40.5925, true, false),
    apt!("SXZ", "LTCL", "Sêrt (Siirt) Airport", "Sêrt", "Bakur", 37.9789, 41.8404, true, false),
    apt!("AJI", "LTCO", "Agirî Ahmed-i Hani Airport", "Agirî", "Bakur", 39.6545, 43.0271, true, false),
    apt!("IGD", "LTCT", "Îdir (Iğdır) Airport", "Îdir", "Bakur", 39.9831, 43.8664, true, false),
    apt!("KSY", "LTCF", "Qers (Kars) Harakani Airport", "Qers", "Bakur", 40.5622, 43.1150, true, false),
    apt!("KCM", "LTCN", "Meraş (Kahramanmaraş) Airport", "Meraş", "Bakur", 37.5390, 36.9534, true, false),

    // ── Rojhilat (Eastern Kurdistan & Luristan) ─────────────────────────────
    apt!("OMH", "OITR", "Urmiye Airport", "Urmiye", "Rojhilat", 37.6681, 45.0687, true, false),
    apt!("SDG", "OICS", "Sine (Sanandaj) Airport", "Sine", "Rojhilat", 35.2459, 47.0092, true, false),
    apt!("KSH", "OICC", "Kirmaşan (Kermanshah) Airport", "Kirmaşan", "Rojhilat", 34.3459, 47.1581, true, false),
    apt!("IIL", "OICI", "Îlam Airport", "Îlam", "Rojhilat", 33.5866, 46.4048, true, false),
    apt!("KHD", "OICK", "Xurremabad Airport", "Xurremabad", "Luristan", 33.4354, 48.2829, true, false),
    apt!("KHY", "OITK", "Xoy (Khoy) Airport", "Xoy", "Rojhilat", 38.4275, 44.9736, true, false),

    // ── Surrounding Regional & International Hubs ───────────────────────────
    apt!("OSM", "ORBM", "Mosul International Airport", "Mosul", "Iraq", 36.3058, 43.1474, false, false),
    apt!("BGW", "ORBI", "Baghdad International Airport", "Baghdad", "Iraq", 33.2625, 44.2346, false, false),
    apt!("NJF", "ORNI", "Al Najaf International Airport", "Najaf", "Iraq", 31.9899, 44.4043, false, false),
    apt!("BSR", "ORMM", "Basra International Airport", "Basra", "Iraq", 30.5491, 47.6621, false, false),
    apt!("TBZ", "OITT", "Tabriz International Airport", "Tabriz", "Iran", 38.1339, 46.2350, false, false),
    apt!("THR", "OIII", "Mehrabad International Airport", "Tehran", "Iran", 35.6892, 51.3134, false, false),
    apt!("IKA", "OIIE", "Imam Khomeini International Airport", "Tehran", "Iran", 35.4161, 51.1522, false, false),
    apt!("MHD", "OIMM", "Mashhad International Airport", "Mashhad", "Iran", 36.2352, 59.6410, false, false),
    apt!("IST", "LTFM", "Istanbul Airport", "Istanbul", "Turkey", 41.2753, 28.7519, false, false),
    apt!("SAW", "LTFJ", "Sabiha Gökçen Airport", "Istanbul", "Turkey", 40.8986, 29.3092, false, false),
    apt!("ESB", "LTAC", "Ankara Esenboğa Airport", "Ankara", "Turkey", 40.1281, 32.9951, false, false),
    apt!("AYT", "LTAI", "Antalya Airport", "Antalya", "Turkey", 36.8987, 30.8005, false, false),
    apt!("ALP", "OSAP", "Aleppo International Airport", "Aleppo", "Syria", 36.1807, 37.2244, false, false),
    apt!("DAM", "OSDI", "Damascus International Airport", "Damascus", "Syria", 33.4115, 36.5156, false, false),
    apt!("EVN", "UDYZ", "Zvartnots International Airport", "Yerevan", "Armenia", 40.1473, 44.3959, false, false),
    apt!("TBS", "UGTB", "Tbilisi International Airport", "Tbilisi", "Georgia", 41.6692, 44.9547, false, false),
    apt!("GYD", "UBBB", "Heydar Aliyev International Airport", "Baku", "Azerbaijan", 40.4675, 50.0467, false, false),
    apt!("DXB", "OMDB", "Dubai International Airport", "Dubai", "UAE", 25.2532, 55.3657, false, false),
    apt!("SHJ", "OMSJ", "Sharjah International Airport", "Sharjah", "UAE", 25.3286, 55.5172, false, false),
    apt!("AUH", "OMAA", "Zayed International Airport", "Abu Dhabi", "UAE", 24.4330, 54.6511, false, false),
    apt!("DOH", "OTHH", "Hamad International Airport", "Doha", "Qatar", 25.2731, 51.6081, false, false),
    apt!("KWI", "OKBK", "Kuwait International Airport", "Kuwait City", "Kuwait", 29.2266, 47.9689, false, false),
    apt!("BAH", "OBBI", "Bahrain International Airport", "Manama", "Bahrain", 26.2708, 50.6336, false, false),
    apt!("AMM", "OJAI", "Queen Alia International Airport", "Amman", "Jordan", 31.7226, 35.9932, false, false),
    apt!("BEY", "OLBA", "Beirut–Rafic Hariri Airport", "Beirut", "Lebanon", 33.8209, 35.4884, false, false),
    apt!("CAI", "HECA", "Cairo International Airport", "Cairo", "Egypt", 30.1219, 31.4056, false, false),
    apt!("JED", "OEJN", "King Abdulaziz International Airport", "Jeddah", "Saudi Arabia", 21.6796, 39.1565, false, false),
    apt!("RUH", "OERK", "King Khalid International Airport", "Riyadh", "Saudi Arabia", 24.9576, 46.6988, false, false),
    apt!("FRA", "EDDF", "Frankfurt Airport", "Frankfurt", "Germany", 50.0379, 8.5622, false, false),
    apt!("DUS", "EDDL", "Düsseldorf Airport", "Düsseldorf", "Germany", 51.2895, 6.7668, false, false),
    apt!("MUC", "EDDM", "Munich Airport", "Munich", "Germany", 48.3538, 11.7861, false, false),
    apt!("BER", "EDDB", "Berlin Brandenburg Airport", "Berlin", "Germany", 52.3667, 13.5033, false, false),
    apt!("VIE", "LOWW", "Vienna International Airport", "Vienna", "Austria", 48.1103, 16.5697, false, false),
    apt!("LHR", "EGLL", "Heathrow Airport", "London", "United Kingdom", 51.4700, -0.4543, false, false),
    apt!("CDG", "LFPG", "Charles de Gaulle Airport", "Paris", "France", 49.0097, 2.5479, false, false),
    apt!("AMS", "EHAM", "Amsterdam Schiphol", "Amsterdam", "Netherlands", 52.3105, 4.7683, false, false),
    apt!("ARN", "ESSA", "Stockholm Arlanda", "Stockholm", "Sweden", 59.6498, 17.9238, false, false),
    apt!("CPH", "EKCH", "Copenhagen Airport", "Copenhagen", "Denmark", 55.6180, 12.6560, false, false),
    apt!("DEL", "VIDP", "Indira Gandhi International Airport", "Delhi", "India", 28.5562, 77.1000, false, false),
    apt!("BOM", "VABB", "Chhatrapati Shivaji Airport", "Mumbai", "India", 19.0896, 72.8656, false, false),
];

/// Finds an airport by IATA or ICAO code.
pub fn airport(code: &str) -> Option<&'static Airport> {
    let code = code.trim();
    if code.is_empty() {
        return None;
    }
    AIRPORTS
        .iter()
        .find(|a| a.iata.eq_ignore_ascii_case(code) || a.icao.eq_ignore_ascii_case(code))
}

pub fn airport_city(code: &str) -> String {
    airport(code)
        .map(|a| a.city.to_string())
        .unwrap_or_else(|| code.to_string())
}

#[derive(Debug, Clone, Copy)]
pub struct Airline {
    pub icao: &'static str,
    pub iata: &'static str,
    pub name: &'static str,
    pub country: &'static str,
    pub military: bool,
}

macro_rules! al {
    ($icao:literal, $iata:literal, $name:literal, $country:literal, $mil:literal) => {
        Airline {
            icao: $icao,
            iata: $iata,
            name: $name,
            country: $country,
            military: $mil,
        }
    };
}

pub static AIRLINES: &[Airline] = &[
    al!("FEG", "HW", "Fly Erbil", "Kurdistan Region", false),
    al!("IAW", "IA", "Iraqi Airways", "Iraq", false),
    al!("URB", "UD", "UR Airlines", "Iraq", false),
    al!("MND", "IF", "Fly Baghdad", "Iraq", false),
    al!("THY", "TK", "Turkish Airlines", "Turkey", false),
    al!("PGT", "PC", "Pegasus Airlines", "Turkey", false),
    al!("TKJ", "VF", "AJet", "Turkey", false),
    al!("SXS", "XQ", "SunExpress", "Turkey", false),
    al!("FDB", "FZ", "flydubai", "UAE", false),
    al!("UAE", "EK", "Emirates", "UAE", false),
    al!("ETD", "EY", "Etihad Airways", "UAE", false),
    al!("ABY", "G9", "Air Arabia", "UAE", false),
    al!("QTR", "QR", "Qatar Airways", "Qatar", false),
    al!("KAC", "KU", "Kuwait Airways", "Kuwait", false),
    al!("JZR", "J9", "Jazeera Airways", "Kuwait", false),
    al!("GFA", "GF", "Gulf Air", "Bahrain", false),
    al!("SVA", "SV", "Saudia", "Saudi Arabia", false),
    al!("FAD", "F3", "flyadeal", "Saudi Arabia", false),
    al!("KNE", "XY", "flynas", "Saudi Arabia", false),
    al!("OMA", "WY", "Oman Air", "Oman", false),
    al!("RJA", "RJ", "Royal Jordanian", "Jordan", false),
    al!("MEA", "ME", "Middle East Airlines", "Lebanon", false),
    al!("MSR", "MS", "EgyptAir", "Egypt", false),
    al!("IRA", "IR", "Iran Air", "Iran", false),
    al!("IRB", "B9", "Iran Airtour", "Iran", false),
    al!("IRC", "EP", "Iran Aseman Airlines", "Iran", false),
    al!("TBZ", "I3", "ATA Airlines", "Iran", false),
    al!("IRM", "W5", "Mahan Air", "Iran", false),
    al!("QSM", "QB", "Qeshm Air", "Iran", false),
    al!("VRH", "VR", "Varesh Airlines", "Iran", false),
    al!("IZG", "ZV", "Zagros Airlines", "Iran", false),
    al!("AIC", "AI", "Air India", "India", false),
    al!("IGO", "6E", "IndiGo", "India", false),
    al!("DLH", "LH", "Lufthansa", "Germany", false),
    al!("CFG", "DE", "Condor", "Germany", false),
    al!("EWG", "EW", "Eurowings", "Germany", false),
    al!("AUA", "OS", "Austrian Airlines", "Austria", false),
    al!("WZZ", "W6", "Wizz Air", "Hungary", false),
    al!("BAW", "BA", "British Airways", "United Kingdom", false),
    al!("AFR", "AF", "Air France", "France", false),
    al!("KLM", "KL", "KLM", "Netherlands", false),
    al!("SEH", "GQ", "Sky Express", "Greece", false),
    al!("AFL", "SU", "Aeroflot", "Russia", false),
    al!("AZG", "7L", "Silk Way West", "Azerbaijan", false),
    al!("AHY", "J2", "Azerbaijan Airlines", "Azerbaijan", false),
    al!("CPA", "CX", "Cathay Pacific", "Hong Kong", false),
    al!("SIA", "SQ", "Singapore Airlines", "Singapore", false),
    // Military / Government / Coalition Operators
    al!("RCH", "", "US Air Force (AMC)", "United States", true),
    al!("CNV", "", "US Navy Logistics", "United States", true),
    al!("LAGR", "", "USAF Tanker Task Force", "United States", true),
    al!("RRR", "", "Royal Air Force (RAF)", "United Kingdom", true),
    al!("GAF", "", "German Air Force (Luftwaffe)", "Germany", true),
    al!("CTM", "", "French Air Force (COTAM)", "France", true),
    al!("IAM", "", "Italian Air Force", "Italy", true),
    al!("NAF", "", "Royal Netherlands Air Force", "Netherlands", true),
    al!("NATO", "", "NATO Airborne Early Warning", "NATO", true),
    al!("UNO", "", "United Nations Aviation", "United Nations", true),
    al!("HVY", "", "Turkish Air Force", "Turkey", true),
    al!("IQAF", "", "Iraqi Air Force", "Iraq", true),
];

/// Resolves an airline from a callsign or flight number such as `THY771` or `TK771`.
pub fn airline_for(callsign: &str) -> Option<&'static Airline> {
    let cs = callsign.trim().to_ascii_uppercase();
    if cs.len() >= 3 {
        if let Some(a) = AIRLINES.iter().find(|a| cs.starts_with(a.icao)) {
            return Some(a);
        }
    }
    if cs.len() >= 3 && cs.as_bytes()[2].is_ascii_digit() {
        return AIRLINES
            .iter()
            .find(|a| !a.iata.is_empty() && cs.starts_with(a.iata));
    }
    None
}

#[derive(Debug, Clone, Copy)]
pub struct AircraftType {
    pub code: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub widebody: bool,
    pub military: bool,
}

macro_rules! ac {
    ($c:literal, $n:literal, $cat:literal, $w:literal, $m:literal) => {
        AircraftType {
            code: $c,
            name: $n,
            category: $cat,
            widebody: $w,
            military: $m,
        }
    };
}

pub static AIRCRAFT: &[AircraftType] = &[
    // Commercial Narrowbody / Regional
    ac!("A19N", "Airbus A319neo", "Narrowbody", false, false),
    ac!("A319", "Airbus A319", "Narrowbody", false, false),
    ac!("A320", "Airbus A320", "Narrowbody", false, false),
    ac!("A20N", "Airbus A320neo", "Narrowbody", false, false),
    ac!("A321", "Airbus A321", "Narrowbody", false, false),
    ac!("A21N", "Airbus A321neo", "Narrowbody", false, false),
    ac!("BCS3", "Airbus A220-300", "Narrowbody", false, false),
    ac!("B737", "Boeing 737-700", "Narrowbody", false, false),
    ac!("B738", "Boeing 737-800", "Narrowbody", false, false),
    ac!("B739", "Boeing 737-900", "Narrowbody", false, false),
    ac!("B38M", "Boeing 737 MAX 8", "Narrowbody", false, false),
    ac!("B39M", "Boeing 737 MAX 9", "Narrowbody", false, false),
    ac!("B752", "Boeing 757-200", "Narrowbody", false, false),
    ac!("E190", "Embraer E190", "Regional Jet", false, false),
    ac!("E195", "Embraer E195", "Regional Jet", false, false),
    ac!("CRJ9", "Bombardier CRJ-900", "Regional Jet", false, false),
    ac!("AT76", "ATR 72-600", "Turboprop", false, false),
    ac!("AT72", "ATR 72", "Turboprop", false, false),
    ac!("DH8D", "De Havilland Dash 8-400", "Turboprop", false, false),

    // Commercial Widebody
    ac!("A310", "Airbus A310", "Widebody", true, false),
    ac!("A332", "Airbus A330-200", "Widebody", true, false),
    ac!("A333", "Airbus A330-300", "Widebody", true, false),
    ac!("A339", "Airbus A330-900neo", "Widebody", true, false),
    ac!("A359", "Airbus A350-900", "Widebody", true, false),
    ac!("A35K", "Airbus A350-1000", "Widebody", true, false),
    ac!("A388", "Airbus A380-800", "Super Heavy", true, false),
    ac!("B763", "Boeing 767-300", "Widebody", true, false),
    ac!("B772", "Boeing 777-200", "Widebody", true, false),
    ac!("B77L", "Boeing 777-200LR", "Widebody", true, false),
    ac!("B77W", "Boeing 777-300ER", "Widebody", true, false),
    ac!("B788", "Boeing 787-8 Dreamliner", "Widebody", true, false),
    ac!("B789", "Boeing 787-9 Dreamliner", "Widebody", true, false),
    ac!("B78X", "Boeing 787-10 Dreamliner", "Widebody", true, false),
    ac!("B744", "Boeing 747-400", "Heavy Cargo/Pax", true, false),
    ac!("B748", "Boeing 747-8", "Heavy Cargo/Pax", true, false),

    // VIP / Government / Business Jets
    ac!("GLF5", "Gulfstream V (Gov/VIP)", "VIP / Gov Jet", false, false),
    ac!("GLF6", "Gulfstream G650", "VIP / Gov Jet", false, false),
    ac!("CL60", "Bombardier Challenger 600", "Special Mission / VIP", false, false),
    ac!("C172", "Cessna 172 Skyhawk", "Light Utility", false, false),

    // Military Transport / Tanker / ISR / Rotorcraft / UAV
    ac!("C130", "Lockheed C-130 Hercules", "Military Transport", false, true),
    ac!("C30J", "Lockheed C-130J Super Hercules", "Military Transport", false, true),
    ac!("C17", "Boeing C-17 Globemaster III", "Heavy Military Airlift", true, true),
    ac!("A400", "Airbus A400M Atlas", "Military Airlift", true, true),
    ac!("K35R", "Boeing KC-135R Stratotanker", "Aerial Refueling", true, true),
    ac!("KC46", "Boeing KC-46A Pegasus", "Aerial Refueling", true, true),
    ac!("E3TF", "Boeing E-3A Sentry AWACS", "Airborne Early Warning", true, true),
    ac!("E737", "Boeing E-7 Wedgetail AEW&C", "Airborne Early Warning", false, true),
    ac!("R135", "Boeing RC-135 Rivet Joint", "Strategic ISR", true, true),
    ac!("P8", "Boeing P-8A Poseidon", "Maritime / Recon", false, true),
    ac!("B350", "Beechcraft King Air 350ER ISR", "Tactical ISR", false, true),
    ac!("C295", "Airbus C-295M", "Tactical Transport", false, true),
    ac!("CN35", "CASA CN-235", "Tactical Transport", false, true),
    ac!("IL76", "Ilyushin Il-76MD", "Heavy Airlift", true, true),
    ac!("H60", "Sikorsky UH-60 Black Hawk", "Tactical Helicopter", false, true),
    ac!("S70", "Sikorsky S-70 Black Hawk", "Tactical Helicopter", false, true),
    ac!("H47", "Boeing CH-47 Chinook", "Heavy Lift Helicopter", false, true),
    ac!("MI8", "Mil Mi-8 / Mi-17", "Utility Helicopter", false, true),
    ac!("Q4", "Northrop Grumman RQ-4 Global Hawk", "High-Altitude UAV", false, true),
    ac!("Q9", "General Atomics MQ-9 Reaper", "Tactical UAV", false, true),
    ac!("TB2", "Baykar Bayraktar TB2", "Tactical UAV", false, true),
    ac!("AKNC", "Baykar Akıncı", "Tactical UAV", false, true),
    ac!("ANKA", "TAI Anka", "Tactical UAV", false, true),
    ac!("F16", "General Dynamics F-16 Fighting Falcon", "Multirole Fighter", false, true),
    ac!("F15", "McDonnell Douglas F-15 Eagle", "Air Superiority Fighter", false, true),
    ac!("F35", "Lockheed Martin F-35 Lightning II", "Stealth Fighter", false, true),
    ac!("EUFI", "Eurofighter Typhoon", "Multirole Fighter", false, true),
    ac!("RFAL", "Dassault Rafale", "Multirole Fighter", false, true),
];

pub fn aircraft(code: &str) -> Option<&'static AircraftType> {
    let code = code.trim();
    AIRCRAFT.iter().find(|a| a.code.eq_ignore_ascii_case(code))
}

/// Identifies military, coalition, government, ISR, or special-mission traffic.
pub fn is_military_or_gov(callsign: &str, model: &str, squawk: &str) -> bool {
    if let Some(ac) = aircraft(model) {
        if ac.military {
            return true;
        }
    }
    if let Some(al) = airline_for(callsign) {
        if al.military {
            return true;
        }
    }
    let cs = callsign.trim().to_ascii_uppercase();
    const MIL_PREFIXES: &[&str] = &[
        "RCH", "CNV", "LAGR", "DUKE", "NCHO", "FORTE", "HOMER", "JAKE", "PYTHON", "RRR", "ASCOT",
        "GAF", "CTM", "FAF", "IAM", "NAF", "NATO", "OTAN", "UNO", "HVY", "TUAF", "THK", "IQAF",
        "IRAF", "KRD", "KRG", "PESH", "VIP", "GOV",
    ];
    if MIL_PREFIXES.iter().any(|p| cs.starts_with(p)) {
        return true;
    }
    let m = model.trim().to_ascii_uppercase();
    if matches!(
        m.as_str(),
        "C130" | "C30J" | "C17" | "A400" | "K35R" | "E3TF" | "R135" | "H60" | "S70" | "H47" | "Q4" | "Q9" | "TB2" | "AKNC" | "ANKA" | "F16" | "F15" | "EUFI"
    ) {
        return true;
    }
    let sq = squawk.trim();
    matches!(sq, "7777" | "4000" | "0000")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn airline_lookup() {
        assert_eq!(airline_for("THY771").unwrap().name, "Turkish Airlines");
        assert_eq!(airline_for("TK2237").unwrap().name, "Turkish Airlines");
        assert_eq!(airline_for("FZ1840").unwrap().name, "flydubai");
        assert_eq!(airline_for("FEG301").unwrap().name, "Fly Erbil");
        assert!(airline_for("ABCDEF").is_none());
    }

    #[test]
    fn airport_lookup() {
        assert_eq!(airport("ORER").unwrap().iata, "EBL");
        assert!(airport("ebl").unwrap().kurdish);
        assert!(airport("LTCE").unwrap().kurdish); // Erzirom
        assert!(airport("OICK").unwrap().kurdish); // Xurremabad
    }
}

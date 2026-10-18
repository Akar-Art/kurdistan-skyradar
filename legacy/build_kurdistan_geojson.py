import json
import math

def simplify_coords(coords, precision=4):
    """Recursively round coordinates to reduce file size."""
    if isinstance(coords[0], (int, float)):
        return [round(coords[0], precision), round(coords[1], precision)]
    return [simplify_coords(c, precision) for c in coords]

def build_datasets():
    # 1. Load IRQ
    with open('irq_adm1.geojson', 'r', encoding='utf-8') as f:
        irq = json.load(f)

    # 2. Load TUR
    with open('tur_adm1.geojson', 'r', encoding='utf-8') as f:
        tur = json.load(f)

    # 3. Load SYR
    with open('syr_adm1.geojson', 'r', encoding='utf-8') as f:
        syr = json.load(f)

    # 4. Load IRN
    with open('irn_adm1.geojson', 'r', encoding='utf-8') as f:
        irn = json.load(f)

    kri_names = {'Erbil', 'Al-Sulaimaniyah', 'Dohuk'}
    bashur_names = {'Erbil', 'Al-Sulaimaniyah', 'Dohuk', 'Kirkuk'}

    bakur_names = {
        'Diyarbakr', 'Diyarbakır', 'Van', 'Mardin', 'Batman', 'Srnak', 'Şırnak', 'Sirnak',
        'Hakkri', 'Hakkâri', 'Hakkari', 'Agr', 'Ağrı', 'Agri', 'Bitlis', 'Siirt',
        'Mus', 'Muş', 'Bingl', 'Bingöl', 'Bingol', 'Tunceli', 'Igdr', 'Iğdır', 'Igdir',
        'Kars', 'Sanlurfa', 'Şanlıurfa', 'Sanliurfa', 'Adyaman', 'Adıyaman', 'Adiyaman',
        'Gaziantep', 'Elazg', 'Elazığ', 'Elazig', 'Malatya', 'Ardahan', 'Kilis'
    }

    rojava_names = {'Al-Hasakeh'}
    rojhelat_names = {'Kurdistan', 'West Azerbaijan', 'Kermanshah', 'Ilam'}

    # Kurdistan Region of Iraq (KRI) Features
    kri_features = []
    for f in irq['features']:
        name = f['properties']['shapeName']
        if name in kri_names:
            geom = f['geometry']
            geom['coordinates'] = simplify_coords(geom['coordinates'], 4)
            kri_features.append({
                "type": "Feature",
                "properties": {
                    "name": name,
                    "kurdish_name": "Hewlêr" if name == "Erbil" else ("Silêmanî" if name == "Al-Sulaimaniyah" else "Dihok"),
                    "part": "Başûr (Southern Kurdistan)",
                    "type": "KRI"
                },
                "geometry": geom
            })

    # Greater Kurdistan Features (all 4 parts)
    greater_features = []

    # Bashur
    for f in irq['features']:
        name = f['properties']['shapeName']
        if name in bashur_names:
            geom = f['geometry']
            geom['coordinates'] = simplify_coords(geom['coordinates'], 4)
            krd_name = "Hewlêr" if name == "Erbil" else ("Silêmanî" if name == "Al-Sulaimaniyah" else ("Dihok" if name == "Dohuk" else "Kerkûk"))
            greater_features.append({
                "type": "Feature",
                "properties": {
                    "name": name,
                    "kurdish_name": krd_name,
                    "part": "Başûr (South Kurdistan)",
                    "region": "Kurdistan"
                },
                "geometry": geom
            })

    # Bakur
    for f in tur['features']:
        name = f['properties']['shapeName']
        for bn in bakur_names:
            if name.lower().replace('ı', 'i').replace('ş', 's').replace('ğ', 'g').replace('ö', 'o').replace('ü', 'u') == bn.lower().replace('ı', 'i').replace('ş', 's').replace('ğ', 'g').replace('ö', 'o').replace('ü', 'u'):
                geom = f['geometry']
                geom['coordinates'] = simplify_coords(geom['coordinates'], 4)
                greater_features.append({
                    "type": "Feature",
                    "properties": {
                        "name": name,
                        "part": "Bakûr (North Kurdistan)",
                        "region": "Kurdistan"
                    },
                    "geometry": geom
                })
                break

    # Rojava
    for f in syr['features']:
        name = f['properties']['shapeName']
        if name in rojava_names:
            geom = f['geometry']
            geom['coordinates'] = simplify_coords(geom['coordinates'], 4)
            greater_features.append({
                "type": "Feature",
                "properties": {
                    "name": name,
                    "kurdish_name": "Cizîrê / Qamişlo / Hesêkê",
                    "part": "Rojava (West Kurdistan)",
                    "region": "Kurdistan"
                },
                "geometry": geom
            })

    # Rojhelat
    for f in irn['features']:
        name = f['properties']['shapeName']
        if name in rojhelat_names:
            geom = f['geometry']
            geom['coordinates'] = simplify_coords(geom['coordinates'], 4)
            krd_name = "Sine / Kordestan" if name == "Kurdistan" else ("Wrmê / Mehabad" if name == "West Azerbaijan" else name)
            greater_features.append({
                "type": "Feature",
                "properties": {
                    "name": name,
                    "kurdish_name": krd_name,
                    "part": "Rojhilat (East Kurdistan)",
                    "region": "Kurdistan"
                },
                "geometry": geom
            })

    # Output unified Kurdistan Atlas GeoJSON
    out_geojson = {
        "type": "FeatureCollection",
        "name": "Kurdistan Atlas Boundaries",
        "kri_provinces": {
            "type": "FeatureCollection",
            "features": kri_features
        },
        "greater_kurdistan": {
            "type": "FeatureCollection",
            "features": greater_features
        }
    }

    with open('kurdistan_atlas.geojson', 'w', encoding='utf-8') as out:
        json.dump(out_geojson, out, ensure_ascii=False)

    print(f"Generated kurdistan_atlas.geojson: {len(kri_features)} KRI provinces, {len(greater_features)} Greater Kurdistan provinces.")

if __name__ == '__main__':
    build_datasets()

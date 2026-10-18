import re

with open('main_atlas.js', 'r', encoding='utf-8', errors='ignore') as f:
    c = f.read()

urls = re.findall(r'https?://[^\s"\'<>)]+', c)
with open('extracted_urls.txt', 'w', encoding='utf-8') as out:
    for u in urls:
        out.write(u + '\n')

    # Search for json/pmtiles/tiles
    matches = re.findall(r'["\']([^"\']*\.(?:json|geojson|pmtiles|mbtiles|pbf))["\']', c)
    out.write('\n--- MATCHES ---\n')
    for m in set(matches):
        out.write(m + '\n')

    # Look for fetch or api calls
    fetches = re.findall(r'fetch\(["\']([^"\']+)["\']\)', c)
    out.write('\n--- FETCHES ---\n')
    for ft in set(fetches):
        out.write(ft + '\n')

print("Extraction completed.")

import json

with open('provs_output.txt', 'w', encoding='utf-8') as out:
    for name, filename in [('TUR', 'tur_adm1.geojson'), ('SYR', 'syr_adm1.geojson'), ('IRN', 'irn_adm1.geojson'), ('IRQ', 'irq_adm1.geojson')]:
        with open(filename, 'r', encoding='utf-8') as f:
            d = json.load(f)
        out.write(f"=== {name} ({len(d['features'])} provinces) ===\n")
        provs = [f['properties']['shapeName'] for f in d['features']]
        out.write(str(provs) + '\n\n')

print("Provinces exported to provs_output.txt")

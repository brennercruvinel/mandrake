import json

with open("config.json") as f:
    c = json.load(f)
print(f"{c['host']}:{c['port']} debug={c['debug']}")

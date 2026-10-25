import json

with open("config.json") as f:
    c = json.load(f)
print(f"{c['server']['host']}:{c['server']['port']} debug={c['flags']['debug']}")

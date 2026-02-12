#!/usr/bin/env bash
# Activate a tenant after purchase — starts their SQ instance
set -euo pipefail

CONFIG="${SQ_ROUTER_CONFIG:-/source/SQ/gateway/founding-config.json}"
SQ_BIN="${SQ_BIN:-sq}"
NAME="${1:?Usage: $0 <tenant-name>}"

# Extract tenant info from config
TOKEN=$(python3 -c "
import json, sys
cfg = json.load(open('$CONFIG'))
for t in cfg['tenants']:
    if t['name'] == '$NAME':
        print(t['token']); sys.exit(0)
print('NOT_FOUND'); sys.exit(1)
") || { echo "Tenant $NAME not found"; exit 1; }

PORT=$(python3 -c "
import json
cfg = json.load(open('$CONFIG'))
for t in cfg['tenants']:
    if t['name'] == '$NAME': print(t['port'])
")

DATA_DIR=$(python3 -c "
import json
cfg = json.load(open('$CONFIG'))
for t in cfg['tenants']:
    if t['name'] == '$NAME': print(t['data_dir'])
")

echo "Activating $NAME on port $PORT..."
nohup $SQ_BIN host "$PORT" --key "$TOKEN" --data-dir "$DATA_DIR" > "/tmp/sq-$NAME.log" 2>&1 &
echo "Started SQ for $NAME (pid=$!, port=$PORT)"

# Mark active in config
python3 -c "
import json
cfg = json.load(open('$CONFIG'))
for t in cfg['tenants']:
    if t['name'] == '$NAME': t['active'] = True
json.dump(cfg, open('$CONFIG', 'w'), indent=2)
"
echo "Marked $NAME as active"

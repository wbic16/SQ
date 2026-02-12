#!/usr/bin/env bash
# Provision 500 SQ tenant slots (ports 1338-1837)
# Tokens are generated but instances only start when purchased
set -euo pipefail

CONFIG="${1:-/source/SQ/gateway/founding-config.json}"
DATA_ROOT="${SQ_DATA_ROOT:-/var/lib/sq/tenants}"
BASE_PORT=1338
COUNT=500
SQ_BIN="${SQ_BIN:-sq}"

echo "=== Provisioning $COUNT SQ tenant slots ==="
mkdir -p "$DATA_ROOT"

# Start with empty tenants array
echo '{"tenants":[' > "$CONFIG"

for i in $(seq 1 $COUNT); do
    PORT=$((BASE_PORT + i - 1))
    TOKEN=$(head -c 32 /dev/urandom | base64 | tr -d '/+=' | head -c 40)
    NAME="tenant-$(printf '%04d' $i)"
    
    # Create data dir
    mkdir -p "$DATA_ROOT/$NAME"
    
    # Append to config
    if [ "$i" -gt 1 ]; then echo ',' >> "$CONFIG"; fi
    cat >> "$CONFIG" <<EOF
{"token":"$TOKEN","port":$PORT,"data_dir":"$DATA_ROOT/$NAME","name":"$NAME","active":false}
EOF

    if [ $((i % 50)) -eq 0 ]; then
        echo "  Provisioned $i/$COUNT slots (port $BASE_PORT-$PORT)"
    fi
done

echo ']}' >> "$CONFIG"
echo ""
echo "=== Done: $COUNT slots provisioned ==="
echo "Config: $CONFIG"
echo "Ports: $BASE_PORT-$((BASE_PORT + COUNT - 1))"
echo "Activate tenants as they purchase via activate-tenant.sh"

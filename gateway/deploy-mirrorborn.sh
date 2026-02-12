#!/usr/bin/env bash
# Deploy SQ Gateway to mirrorborn.us
# Run on the target machine (Verse pulls this via git)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SQ_BIN="${SQ_BIN:-sq}"
DATA_ROOT="/var/lib/sq-tenants"
CONFIG="$SCRIPT_DIR/sq-gateway.toml"
GW_PORT=8080

echo "=== SQ Gateway Deployment ==="

# Create data directory
sudo mkdir -p "$DATA_ROOT"
sudo chown "$(whoami)" "$DATA_ROOT"

# Generate production config if not exists
if [ ! -f "$CONFIG" ]; then
    ADMIN_KEY=$(head -c 32 /dev/urandom | base64 | tr -d '/+=' | head -c 40)
    cat > "$CONFIG" <<EOF
[gateway]
listen_port = $GW_PORT
admin_key = "$ADMIN_KEY"
EOF
    echo "Created config with admin_key: $ADMIN_KEY"
    echo "SAVE THIS KEY — it won't be shown again."
else
    echo "Config exists at $CONFIG"
fi

# Create systemd service for gateway
sudo tee /etc/systemd/system/sq-gateway.service > /dev/null <<EOF
[Unit]
Description=SQ Auth Gateway
After=network.target

[Service]
Type=simple
User=$(whoami)
WorkingDirectory=$SCRIPT_DIR
Environment=SQ_GATEWAY_CONFIG=$CONFIG
ExecStart=/usr/bin/python3 $SCRIPT_DIR/sq-gateway.py $GW_PORT
ExecReload=/bin/kill -HUP \$MAINPID
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable sq-gateway
sudo systemctl restart sq-gateway

echo ""
echo "Gateway running on port $GW_PORT"
echo "Add tenants with: $SCRIPT_DIR/tenant-manager.sh add <name>"
echo "Reload config:    sudo systemctl reload sq-gateway"
echo "View logs:        journalctl -u sq-gateway -f"

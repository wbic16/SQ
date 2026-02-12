#!/bin/bash
# SQ Cloud Founding Deployment Script
# Deploys router + 5 founding tenant backends

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║       SQ Cloud Founding Deployment (Feb 13, 2026)       ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "❌ Please run as root (sudo)"
    exit 1
fi

# Create sq user if doesn't exist
if ! id -u sq > /dev/null 2>&1; then
    echo "Creating sq user..."
    useradd -r -s /bin/false -d /var/lib/sq sq
fi

# Create data directories
echo "Creating tenant data directories..."
mkdir -p /var/lib/sq/tenants/{wbic16,demo,alpha-001,alpha-002,alpha-003}
mkdir -p /etc/sq/secrets

# Set ownership
chown -R sq:sq /var/lib/sq
chmod 750 /var/lib/sq/tenants/*

# Copy config to secure location
echo "Installing router config..."
cp router-config-founding.json /etc/sq/router-config.json
chown sq:sq /etc/sq/router-config.json
chmod 640 /etc/sq/router-config.json

# Copy founding tokens to secure location (DO NOT COMMIT THIS FILE)
if [ -f FOUNDING-TOKENS.md ]; then
    echo "Installing founding tokens (encrypted)..."
    gpg --encrypt --recipient wbic16@gmail.com FOUNDING-TOKENS.md
    mv FOUNDING-TOKENS.md.gpg /etc/sq/secrets/
    chmod 600 /etc/sq/secrets/FOUNDING-TOKENS.md.gpg
    echo "⚠️  Original FOUNDING-TOKENS.md still exists - DELETE IT after verifying encrypted copy"
fi

# Install systemd services
echo "Installing systemd services..."
cp systemd/sq-router.service /etc/systemd/system/
cp systemd/sq-backend-wbic16.service /etc/systemd/system/
cp systemd/sq-backend-demo.service /etc/systemd/system/

# Reload systemd
systemctl daemon-reload

# Enable services (don't start yet)
echo "Enabling services..."
systemctl enable sq-router
systemctl enable sq-backend-wbic16
systemctl enable sq-backend-demo

echo ""
echo "✅ Deployment complete!"
echo ""
echo "Next steps:"
echo "1. Verify SQ binary: /usr/local/bin/sq"
echo "2. Start backends: sudo systemctl start sq-backend-{wbic16,demo}"
echo "3. Start router: sudo systemctl start sq-router"
echo "4. Test: curl -H 'Authorization: pmb-v1-...' http://localhost:1337/select/1.1.1/1.1.1/1.1.1"
echo ""
echo "Service status:"
echo "  sudo systemctl status sq-router"
echo "  sudo systemctl status sq-backend-wbic16"
echo "  sudo systemctl status sq-backend-demo"
echo ""
echo "Logs:"
echo "  sudo journalctl -u sq-router -f"
echo "  sudo journalctl -u sq-backend-wbic16 -f"
echo ""

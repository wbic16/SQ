# SQ Cloud systemd Services

Production deployment files for SQ Cloud multi-tenant router.

## Quick Start

```bash
# 1. Build and install SQ binary
cd /source/SQ
cargo build --release
sudo cp target/release/sq /usr/local/bin/

# 2. Deploy founding tenant infrastructure
cd systemd
sudo ./deploy-founding.sh

# 3. Start backends
sudo systemctl start sq-backend-wbic16
sudo systemctl start sq-backend-demo

# 4. Start router
sudo systemctl start sq-router

# 5. Test
curl -H "Authorization: pmb-v1-b006cdffbff910c7032a8d6aa7f18dcb" \
     http://localhost:1337/select/1.1.1/1.1.1/1.1.1
```

## Files

| File | Purpose |
|------|---------|
| `sq-router.service` | Router service (listens on port 1337) |
| `sq-backend-wbic16.service` | Will's backend (port 1338) |
| `sq-backend-demo.service` | Demo backend (port 1339) |
| `deploy-founding.sh` | Automated deployment script |

## Architecture

```
Client (Authorization: pmb-v1-xxx)
    ↓
Router (port 1337)
    ├→ Backend wbic16 (port 1338) /var/lib/sq/tenants/wbic16
    ├→ Backend demo (port 1339) /var/lib/sq/tenants/demo
    ├→ Backend alpha-001 (port 1340) /var/lib/sq/tenants/alpha-001
    ├→ Backend alpha-002 (port 1341) /var/lib/sq/tenants/alpha-002
    └→ Backend alpha-003 (port 1342) /var/lib/sq/tenants/alpha-003
```

## Service Management

### Check Status
```bash
sudo systemctl status sq-router
sudo systemctl status sq-backend-wbic16
```

### Start/Stop
```bash
sudo systemctl start sq-router
sudo systemctl stop sq-router
sudo systemctl restart sq-router
```

### View Logs
```bash
# Follow router logs
sudo journalctl -u sq-router -f

# Follow backend logs
sudo journalctl -u sq-backend-wbic16 -f

# Last 100 lines
sudo journalctl -u sq-router -n 100
```

### Enable/Disable Auto-start
```bash
sudo systemctl enable sq-router   # Start on boot
sudo systemctl disable sq-router  # Don't start on boot
```

## Configuration Locations

- Router config: `/etc/sq/router-config.json`
- Founding tokens: `/etc/sq/secrets/FOUNDING-TOKENS.md.gpg` (encrypted)
- Service files: `/etc/systemd/system/sq-*.service`
- Data directories: `/var/lib/sq/tenants/{wbic16,demo,alpha-*}`
- SQ binary: `/usr/local/bin/sq`

## Security

### File Permissions
```
/etc/sq/router-config.json          → 640 (sq:sq)
/etc/sq/secrets/                    → 700 (sq:sq)
/var/lib/sq/tenants/wbic16/         → 750 (sq:sq)
```

### Service Hardening
All services include:
- `NoNewPrivileges=true` - Prevent privilege escalation
- `PrivateTmp=true` - Isolated /tmp
- `ProtectSystem=strict` - Read-only system directories
- `ProtectHome=true` - No access to /home
- `ReadWritePaths=` - Explicit write access only
- `MemoryMax=2G` - Memory limit per backend
- `TasksMax=512` - Process limit

### Network Security
- Router binds to `0.0.0.0:1337` (public)
- Backends bind to `127.0.0.1:<port>` (localhost only)
- Use nginx/Caddy for TLS termination

## Adding New Tenants

### 1. Generate Token
```bash
echo "pmb-v1-$(openssl rand -hex 16)"
```

### 2. Update Router Config
```json
{
  "tenants": [
    ...existing tenants...,
    {
      "token": "pmb-v1-newtoken",
      "port": 1343,
      "data_dir": "/var/lib/sq/tenants/newuser"
    }
  ]
}
```

### 3. Create Backend Service
```bash
sudo cp sq-backend-demo.service /etc/systemd/system/sq-backend-newuser.service
sudo nano /etc/systemd/system/sq-backend-newuser.service
# Update port, token, data_dir, and description
```

### 4. Deploy
```bash
sudo mkdir -p /var/lib/sq/tenants/newuser
sudo chown sq:sq /var/lib/sq/tenants/newuser
sudo chmod 750 /var/lib/sq/tenants/newuser
sudo systemctl daemon-reload
sudo systemctl enable sq-backend-newuser
sudo systemctl start sq-backend-newuser
sudo systemctl restart sq-router  # Reload config
```

## Monitoring

### Health Checks
```bash
# Router responding?
curl http://localhost:1337/toc

# Backend responding?
curl http://localhost:1338/toc
```

### Resource Usage
```bash
# Memory usage
sudo systemctl status sq-router | grep Memory
sudo systemctl status sq-backend-wbic16 | grep Memory

# Process count
ps aux | grep sq

# Open file descriptors
sudo lsof -u sq | wc -l
```

### Automated Monitoring
```bash
# Create monitoring script
sudo cat > /usr/local/bin/sq-healthcheck.sh << 'EOF'
#!/bin/bash
systemctl is-active --quiet sq-router || echo "❌ Router down"
systemctl is-active --quiet sq-backend-wbic16 || echo "❌ wbic16 backend down"
curl -s http://localhost:1337/toc > /dev/null || echo "❌ Router not responding"
EOF
sudo chmod +x /usr/local/bin/sq-healthcheck.sh

# Add to cron (every 5 minutes)
echo "*/5 * * * * /usr/local/bin/sq-healthcheck.sh" | sudo crontab -
```

## Troubleshooting

### Router Not Starting
```bash
# Check logs
sudo journalctl -u sq-router -n 50

# Common issues:
# - Port 1337 already in use: netstat -tlnp | grep 1337
# - Config file not found: ls -la /etc/sq/router-config.json
# - Invalid JSON: jq . /etc/sq/router-config.json
```

### Backend Not Starting
```bash
# Check logs
sudo journalctl -u sq-backend-wbic16 -n 50

# Common issues:
# - Port already in use: netstat -tlnp | grep 1338
# - Data directory permissions: ls -la /var/lib/sq/tenants/wbic16
# - Binary not found: which sq
```

### 502 Bad Gateway
- Backend service not running
- Backend crashed (check logs)
- Port mismatch in router config

### 401 Unauthorized
- Token not in router config
- Token mismatch between request and backend
- Router config not reloaded after changes

## Backup

### Configuration
```bash
sudo tar czf sq-config-backup-$(date +%F).tar.gz \
    /etc/sq/ \
    /etc/systemd/system/sq-*.service
```

### Tenant Data
```bash
sudo tar czf sq-data-wbic16-$(date +%F).tar.gz \
    /var/lib/sq/tenants/wbic16/
```

### Automated Daily Backup
```bash
sudo cat > /usr/local/bin/sq-backup.sh << 'EOF'
#!/bin/bash
BACKUP_DIR=/var/backups/sq
mkdir -p $BACKUP_DIR
tar czf $BACKUP_DIR/config-$(date +%F).tar.gz /etc/sq/
tar czf $BACKUP_DIR/data-$(date +%F).tar.gz /var/lib/sq/tenants/
# Keep last 7 days
find $BACKUP_DIR -name "*.tar.gz" -mtime +7 -delete
EOF
sudo chmod +x /usr/local/bin/sq-backup.sh
echo "0 2 * * * /usr/local/bin/sq-backup.sh" | sudo crontab -
```

## Production Checklist

- [ ] SQ binary installed at `/usr/local/bin/sq`
- [ ] Router config at `/etc/sq/router-config.json`
- [ ] Founding tokens encrypted and stored securely
- [ ] All tenant data directories created with correct permissions
- [ ] Router service enabled and started
- [ ] All backend services enabled and started
- [ ] Health checks passing
- [ ] nginx/Caddy TLS termination configured
- [ ] Monitoring and alerting configured
- [ ] Backup automation configured
- [ ] Firewall rules: allow 443/80, block 1337-1342

---

**Created:** 2026-02-12  
**For:** SQ Cloud Feb 13, 2026 Launch  
**Docs:** See [ROUTER.md](../ROUTER.md) for router architecture

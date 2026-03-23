# SQ Auth Gateway v0.5.5

Per-tenant auth proxy for SQ phext database instances.

## Quick Start

```bash
cp sq-gateway.toml.example sq-gateway.toml
# Edit sq-gateway.toml — set admin_key, add tenants
python3 sq-gateway.py          # listens on configured port
python3 sq-gateway.py 8080     # override port via CLI
```

## API Usage

```bash
# Query a tenant's SQ instance
curl -H "Authorization: Bearer <token>" http://localhost:8080/path

# Admin: list tenants
curl -H "Authorization: Bearer <admin_key>" http://localhost:8080/admin/tenants

# Health check (no auth)
curl http://localhost:8080/health
```

## Tenant Management

```bash
chmod +x tenant-manager.sh
./tenant-manager.sh add alice      # generates token, starts SQ
./tenant-manager.sh list           # show all tenants + status
./tenant-manager.sh remove alice   # stops SQ, removes from config
```

## Config

See `sq-gateway.toml.example`. Environment variable `SQ_GATEWAY_CONFIG` overrides default path.

## Requirements

- Python 3.11+ (uses `tomllib` from stdlib)
- SQ binary in PATH (or set `SQ_BIN`)

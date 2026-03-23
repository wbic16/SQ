# SQ Router v0.5.5 - Token-Based Multi-Tenant Routing

## Overview

The SQ Router provides token-based authentication and routing for multi-tenant SQ deployments. Instead of exposing each tenant's SQ instance directly, you run:
- **One router** on a public port (e.g., 443 or 1337)
- **Multiple SQ instances** on private ports, one per tenant

The router reads the `Authorization` header from incoming requests, looks up which backend port serves that token, and proxies the request transparently.

## Architecture

```
Client Request (Authorization: pmb-v1-abc123)
    ↓
Router (port 1337) - reads token from header
    ↓
Backend SQ (port 1338) - serves tenant 1's data
    ↓
Response
```

Each tenant gets:
- Unique `pmb-v1-xxx` authentication token
- Dedicated SQ instance: `sq host <port> --key <token> --data-dir <path>`
- Isolated data directory

## Quick Start

### 1. Create router configuration

```json
{
  "tenants": [
    {
      "token": "pmb-v1-user1-abc123",
      "port": 1338,
      "data_dir": "/var/lib/sq/tenants/user1"
    },
    {
      "token": "pmb-v1-user2-def456",
      "port": 1339,
      "data_dir": "/var/lib/sq/tenants/user2"
    }
  ]
}
```

Save as `router-config.json`.

### 2. Start backend SQ instances (one per tenant)

Terminal 1:
```bash
sq host 1338 --key pmb-v1-user1-abc123 --data-dir /var/lib/sq/tenants/user1
```

Terminal 2:
```bash
sq host 1339 --key pmb-v1-user2-def456 --data-dir /var/lib/sq/tenants/user2
```

### 3. Start the router

```bash
sq route router-config.json 1337
```

### 4. Test with curl

```bash
# User 1 request
curl -H "Authorization: pmb-v1-user1-abc123" \
     http://localhost:1337/select/1.1.1/1.1.1/1.1.1

# User 2 request
curl -H "Authorization: pmb-v1-user2-def456" \
     http://localhost:1337/select/1.1.1/1.1.1/1.1.1
```

Each user sees only their own data.

## Configuration Format

```json
{
  "tenants": [
    {
      "token": "pmb-v1-xxx",    // Auth token (must match backend --key)
      "port": 1338,             // Backend SQ instance port (localhost)
      "data_dir": "/path"       // Tenant data directory
    }
  ]
}
```

**Validation:**
- No duplicate tokens allowed
- Backend ports must be listening before router starts
- Token format: any string (convention: `pmb-v1-<identifier>`)

## Security Features

1. **Token-based auth**: Only requests with valid tokens are routed
2. **Tenant isolation**: Each backend serves one tenant's data directory
3. **Path validation**: Backend SQ prevents directory traversal (`..`, `/`, `\`)
4. **Timeouts**: 30-second timeout on proxy connections
5. **Header limits**: 16 KB max header size

## Usage

```bash
# Start router with default config and port
sq route

# Start router with custom config
sq route my-config.json

# Start router with custom config and port
sq route my-config.json 443
```

**Default values:**
- Config file: `router-config.json`
- Listen port: `1337`

## Production Deployment

### systemd service for router

```ini
[Unit]
Description=SQ Router
After=network.target

[Service]
Type=simple
User=sq
WorkingDir=/var/lib/sq
ExecStart=/usr/local/bin/sq route /etc/sq/router-config.json 1337
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

### systemd service for backend (template)

```ini
[Unit]
Description=SQ Backend for %i
After=network.target

[Service]
Type=simple
User=sq
WorkingDir=/var/lib/sq/tenants/%i
ExecStart=/usr/local/bin/sq host ${PORT} --key ${TOKEN} --data-dir /var/lib/sq/tenants/%i
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Create one instance per tenant:
```bash
systemctl enable sq-backend@user1
systemctl start sq-backend@user1
```

## TLS/HTTPS

The router operates at HTTP level. For HTTPS:

**Option A: nginx reverse proxy**
```nginx
server {
    listen 443 ssl;
    server_name sq.example.com;
    
    ssl_certificate /etc/letsencrypt/live/sq.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/sq.example.com/privkey.pem;
    
    location / {
        proxy_pass http://127.0.0.1:1337;
        proxy_set_header Authorization $http_authorization;
        proxy_set_header Host $host;
    }
}
```

**Option B: Caddy**
```
sq.example.com {
    reverse_proxy localhost:1337
}
```

## API Compatibility

The router is fully transparent - clients use the same SQ API:
- `GET /select/<coord>` - Read scroll
- `POST /insert?library=X&shelf=Y&...` - Write scroll
- `POST /update?library=X&shelf=Y&...` - Update scroll
- `GET /toc` - Table of contents

Only difference: add `Authorization: pmb-v1-xxx` header.

## Token Generation

Generate secure tokens:

```bash
# Random 32-character token
echo "pmb-v1-$(openssl rand -hex 16)"

# Output: pmb-v1-a3f2b8c4d5e6f7a8b9c0d1e2f3a4b5c6
```

Convention: `pmb-v1-<32-hex-chars>`

## Monitoring

Router logs to stdout:
```
╔══════════════════════════════════════════════════════════╗
║             SQ Router v0.5.5 (Token-based)              ║
╚══════════════════════════════════════════════════════════╝

Listening on: 0.0.0.0:1337
Tenants configured: 3
Config file: router-config.json

[1] Routing to backend port 1338
[2] Routing to backend port 1339
[3] Invalid token: pmb-v1-i
[3] Unauthorized - Invalid token
```

## Error Responses

- **401 Unauthorized**: Missing or invalid token
- **400 Bad Request**: Malformed HTTP request
- **502 Bad Gateway**: Backend SQ not responding
- **500 Internal Server Error**: Router error

## Migration from Direct SQ

**Before (direct access):**
```bash
sq host 1337
curl http://localhost:1337/select/1.1.1/1.1.1/1.1.1
```

**After (routed access):**
```bash
# Start backend
sq host 1338 --key pmb-v1-abc123 --data-dir /data/tenant1

# Start router
sq route config.json 1337

# Client adds auth header
curl -H "Authorization: pmb-v1-abc123" \
     http://localhost:1337/select/1.1.1/1.1.1/1.1.1
```

## Limitations

- Router runs single-threaded (one connection at a time)
- Backend SQ instances must be started separately
- Config file not hot-reloaded (restart router to update)
- HTTP only (use nginx/Caddy for HTTPS)

## Future Enhancements

- Multi-threaded request handling
- Hot config reload (SIGHUP)
- Built-in HTTPS support
- Rate limiting per tenant
- Prometheus metrics endpoint
- Backend health checks

## Support

- Issues: https://github.com/wbic16/SQ/issues
- Docs: https://phext.io/
- Discord: https://discord.com/invite/clawd

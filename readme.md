Esque
-----
SQ: The Simple Query Tool

SQ is a modern database, written from the ground-up to take advantage of Phext: 11-dimensional plain hypertext. Like normal SQL databases, it has multiple pronunciations: "esque", "S-Q", "Seek", "Super Quine", and "Self Query".


## Getting Started

you can either clone this repo and run `cargo build`, or just install the latest stable build: `cargo install sq`. You may have also arrived here from Choose Your Own Adventure...

A pre-built x86_64 container is available via Docker as well: `docker pull wbic16/sq`.

## Commands

SQ is designed to keep abstractions to a minimum. You can interact with phexts via shared memory (daemon "esque" mode) or a TCP socket ("seek" mode) with a simple REST API.

* sq help: displays online help
* sq version: displays the current version
* sq share: <file>: launches a server that hosts a phext file via shared memory
* sq status: Displays daemon statistics (loaded phext, size, connection count)
* sq toc: Displays a textmap (list of available scrolls) of the currently-loaded phext
* sq checksum: Displays the checksum of the current phext
* sq delta: Displays the hierarchical network of checksums for the current phext
* sq push <coord> <file>: Overwrites the specified scroll with the local file
* sq pull <coord> <file>: Fetches the specified scroll to a local file
* sq select <coord>: Fetches content from the current phext
* sq insert <coord> "text": Appends text at the specified coordinate
* sq update <coord> "text": Overwrites text at the specified coordinate
* sq delete <coord>: Removes all content from the specified coordinate
* sq save <file>: Writes the current phext back to disk
* sq json-export <file>: Dumps the contents of the current phext as json
* sq init: Fast initialization for hosting world.phext from any state
* sq shutdown: Instruct the daemon to terminate

# Modes of Operation

* `Daemon Mode`: If you supply a filename parameter to sq, it will launch in daemon mode - communicating with local system processes via shared memory
* `Listening Mode`: If you supply a port number to sq, it will launch in web server mode - listening on the TCP socket requested
* `Router Mode` (**NEW in v0.5.5**): Token-based multi-tenant routing layer - see ROUTER.md for details
* `API Proxy Mode` (**NEW in v0.6.0**): OpenAI-compatible prompt proxy - triages each prompt to cache, a local Ollama model, or an upstream provider

## Router Mode (v0.5.5)

The SQ Router enables multi-tenant deployments with token-based authentication and request routing. Instead of exposing each tenant's SQ instance directly, you run:
- One router on a public port (e.g., 443 or 1337)
- Multiple SQ instances on private ports (one per tenant)

The router reads the `Authorization` header, looks up the tenant's backend port, and proxies the request transparently.

**Quick Start:**
```bash
# Start backend SQ instances
sq host 1338 --key pmb-v1-user1-abc123 --data-dir /data/user1
sq host 1339 --key pmb-v1-user2-def456 --data-dir /data/user2

# Start router
sq route router-config.json 1337

# Clients use Authorization header
curl -H "Authorization: pmb-v1-user1-abc123" \
     http://localhost:1337/select/1.1.1/1.1.1/1.1.1
```

**Features:**
- Token-based authentication (pmb-v1 format)
- Per-tenant data isolation
- Path traversal prevention
- Transparent proxy (same SQ API)
- Production-ready (systemd services, TLS via nginx/Caddy)

📖 **Full documentation:** See [ROUTER.md](router.md) for complete setup guide, security features, and production deployment.

## Authentication

When SQ is started with `--key <token>` (or with a tenant config), every request must carry the token in one of these headers:

* `Authorization: <token>` or `Authorization: Bearer <token>`
* `X-SQ-API-Key: <token>` (used by Phext Notepad)

Requests without a valid token receive `401 Unauthorized`. Unknown routes return `404 Not Found` and are logged to stderr with their request line. Clients must include the full base URI, e.g. `http://host:1337/api/v2`.

## API Proxy Mode (v0.6.0)

`sq api <config.json> [port]` (default port 8100) runs an OpenAI-compatible chat endpoint that routes each prompt to the cheapest tier that can handle it:

1. **Cache** - SHA256-keyed prompt cache with LRU eviction and TTL expiry
2. **Local** - an Ollama model (e.g. `qwen3-coder-next`) for formatting, translation, and quick tasks
3. **Upstream** - a hosted provider for complex reasoning, architecture, and phext/Exocortex work

Prompts are scored against local and upstream signal patterns (`src/triage.rs`). If a local request fails it falls back to upstream, and a feedback loop routes all local-tier traffic upstream while the local failure rate over the last `feedback_window` requests exceeds `escalation_threshold`. The full conversation history is forwarded to whichever tier answers; cache lookups are keyed on the last user message.

Config fields: `local_url`, `local_model`, `local_timeout_secs`, `upstream_url`, `upstream_api_key`, `upstream_model`, `cache_max_entries`, `cache_ttl_secs`, `signal_threshold`, `escalation_threshold`, `feedback_window`. Keep `upstream_api_key` out of version control.

## SQ Design Philosophy

SQ is a complete ground-up rewrite of database concepts. It probably doesn't have features you expect from a database. What it does offer is simplicity. SQ is designed to mirror computer architecture in 2025, not 1970. Databases are stored in phext files using variable-length scrolls. Essentially, everything in a phext database is a string.

SQ leverages Rust and libphext-rs to provide the core data store. All database primitives in SQ are built in terms of phext. For more information about the phext encoding format, refer to https://phext.io.

# Developing

In daemon mode, SQ uses shared memory to ensure that data transfers to/from the database engine are done as quickly as possible. The shared memory segments are managed by files stored in the .sq directory where you invoked SQ from. It is expected that you will run the client and the server from the same directory.

In listening mode, SQ reads and writes phexts via REST.

## REST API Endpoint

SQ offers a simple CRUD-style REST API. The API allows you to interact with multiple phexts from CURL or your web browser. Saving is automatic - if a command changes the content of a phext, it will be saved to disk immediately. Note that if you change the loaded phext without issuing a load command, SQ will automatically reload from disk first.

* /api/v2/version: Displays the current version of SQ
* /api/v2/load?p=<phext>: Loads the entire contents of `phext`.phext into the current context
* /api/v2/select?p=<phext>&c=<coordinate>: Fetches the scroll of text found at `coordinate` in `phext`.phext
* /api/v2/insert?p=<phext>&c=<coordinate>&s=<scroll>: Appends a scroll of text at `coordinate` in `phext`.phext
* /api/v2/update?p=<phext>&c=<coordinate>&s=<scroll>: Overwrites the contents of the scroll at `coordinate` in `phext`.phext
* /api/v2/delete?p=<phext>&c=<coordinate>: Clears the contents of the scroll at `coordinate` in `phext`.phext
* /api/v2/delta?p=<phext>: Returns the hierarchical map of checksums for the given phext
* /api/v2/toc?p=<phext>: Returns the table of contents for the given phext
* /api/v2/get?p=<phext>: Returns a complete copy of the given phext
* /api/v2/query?p=<phext>&from=<coordinate>&to=<coordinate>: Returns every non-empty scroll in the inclusive coordinate range (**v0.6.0**)
* /api/v2/query?p=<phext>&set=<coordinate>,<coordinate>,...: Returns the scrolls at an explicit set of coordinates (**v0.6.0**)

## Coordinate Query API (v0.6.0)

Query a phext by coordinate range, by coordinate set, or both at once:

```bash
# Range: every non-empty scroll from the first coordinate to the second, inclusive
curl -H "X-SQ-API-Key: $KEY" "http://localhost:1337/api/v2/query?p=world&from=1.1.1/1.1.1/1.1.1&to=1.1.1/1.1.2/1.1.1"

# Set: specific coordinates (missing ones come back with an empty scroll)
curl -H "X-SQ-API-Key: $KEY" "http://localhost:1337/api/v2/query?p=world&set=1.1.1/1.1.1/1.1.1,2.1.1/1.1.1/1.1.1"

# Large sets: POST the coordinates as the body, separated by commas or newlines
curl -X POST -H "X-SQ-API-Key: $KEY" --data-binary @coords.txt "http://localhost:1337/api/v2/query?p=world"
```

The response is a JSON array of `{"coordinate": "...", "scroll": "..."}`, sorted by coordinate from library down to scroll. When a range and a set are combined, the results are merged with duplicates removed. Add `&limit=N` to cap the number of results. Coordinates must be complete (`a.b.c/d.e.f/g.h.i`, every part at least 1); a malformed coordinate, a range with only one end, or a range whose start comes after its end returns `400 Bad Request` with the reason.

# Trivia

SQ was bundled into CYOA on 6/12/2025 and 7/15/2025.

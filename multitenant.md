# SQ v0.5.5 Multi-Tenant Mode

**Status:** Config module ready, integration pending

---

## What's Included

**Files added:**
- `src/config.rs` - JSON config loading for tenant definitions
- `Cargo.toml` - Updated with serde/serde_json dependencies

**Version:** Bumped to 0.5.5

---

## Integration Status

The config module is ready but not yet integrated with the main server loop due to upstream changes (mesh/router modules added in v0.5.4).

**Two paths forward:**

### Option A: Nginx Router (Deploy Today)
Use the existing v0.5.4 with per-process tenants + nginx routing (already implemented in `/source/exo-plan/rounds/r21/`).

**Pros:** Ships immediately, proven architecture  
**Cons:** N processes for N tenants

### Option B: Integrated Multi-Tenant (1-2 hours)
Integrate `config.rs` with the main server loop to support 500 tenants in one process.

**Changes needed:**
1. Add `--config <path>` flag parsing in `main()`
2. Load `ServerConfig` and pass to connection handler
3. Modify `handle_tcp_connection()` to use config-based auth
4. Adapt to new `send_response()` pattern from v0.5.4

---

##Config Format

```json
{
  "tenants": {
    "pmb-v1-001-abc123": {
      "name": "founding-001",
      "data_dir": "/var/lib/sq/tenants/founding-001"
    },
    "pmb-v1-002-def456": {
      "name": "founding-002",
      "data_dir": "/var/lib/sq/tenants/founding-002"
    }
  }
}
```

**500-tenant config:** Already generated in `/source/exo-plan/rounds/r21/founding-500-tokens.json` (57 KB)

---

## Testing

Multi-tenant mode was tested with a custom build and works correctly:
- ✅ Token-based auth
- ✅ Path traversal protection
- ✅ Tenant data isolation
- ✅ Write/read operations

**Test results preserved in commit message.**

---

## Recommendation

**For R21 launch today:** Use Option A (nginx router + per-process tenants)

**For production scale:** Integrate Option B when ready (supports 500+ tenants efficiently)

The config module is ready and tested - just needs the server loop integration.

---

**Next Steps:**
1. Review upstream changes in v0.5.4 (mesh/router modules)
2. Adapt multi-tenant handler to new error handling patterns
3. Test with 500-tenant config
4. Deploy

**Estimated integration time:** 1-2 hours

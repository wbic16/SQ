#!/usr/bin/env bash
# SQ Tenant Manager — add/remove/list tenants
set -euo pipefail

CONFIG="${SQ_GATEWAY_CONFIG:-$(dirname "$0")/sq-gateway.toml}"
SQ_BIN="${SQ_BIN:-sq}"
DATA_ROOT="${SQ_DATA_ROOT:-/data}"
BASE_PORT=1337

die() { echo "ERROR: $*" >&2; exit 1; }

gen_token() { head -c 32 /dev/urandom | base64 | tr -d '/+=' | head -c 40; }

next_port() {
    local max=$BASE_PORT
    while IFS= read -r p; do
        (( p >= max )) && max=$((p + 1))
    done < <(grep '^port' "$CONFIG" 2>/dev/null | grep -oP '\d+')
    echo "$max"
}

cmd_add() {
    local name="${1:?usage: $0 add <name>}"
    grep -q "name = \"$name\"" "$CONFIG" 2>/dev/null && die "tenant '$name' already exists"
    local token port
    token=$(gen_token)
    port=$(next_port)
    mkdir -p "$DATA_ROOT/$name"
    cat >> "$CONFIG" <<EOF

[[tenants]]
token = "$token"
name = "$name"
port = $port
phext = "$name"
EOF
    echo "Added tenant: $name (port=$port)"
    echo "Token: $token"
    # Start SQ instance
    nohup $SQ_BIN host "$port" --key "$token" --data-dir "$DATA_ROOT/$name" \
        > "/tmp/sq-$name.log" 2>&1 &
    echo "Started SQ on port $port (pid=$!)"
}

cmd_remove() {
    local name="${1:?usage: $0 remove <name>}"
    # Find port and kill
    local port
    port=$(awk "/name = \"$name\"/{found=1} found && /^port/{print \$3; exit}" "$CONFIG")
    [ -n "$port" ] || die "tenant '$name' not found"
    # Kill SQ process on that port
    pkill -f "$SQ_BIN host $port" 2>/dev/null || true
    # Remove tenant block (5 lines: blank + header + token + name + port + phext)
    local tmp
    tmp=$(mktemp)
    awk -v n="$name" '
        /^\[\[tenants\]\]/ { block=1; buf=$0"\n"; next }
        block && /^\[\[/ { block=0; if (!skip) printf "%s", buf; buf=""; skip=0 }
        block { buf=buf $0"\n"; if ($0 ~ "name = \""n"\"") skip=1; next }
        !block { if (!skip) print; skip=0 }
        END { if (block && !skip) printf "%s", buf }
    ' "$CONFIG" > "$tmp"
    mv "$tmp" "$CONFIG"
    echo "Removed tenant: $name"
}

cmd_list() {
    echo "Tenants in $CONFIG:"
    grep 'name = ' "$CONFIG" 2>/dev/null | sed 's/.*= "//;s/"//' | while read -r n; do
        port=$(awk "/name = \"$n\"/{found=1} found && /^port/{print \$3; exit}" "$CONFIG")
        running="stopped"
        pgrep -f "$SQ_BIN host $port" >/dev/null 2>&1 && running="running"
        echo "  $n  port=$port  $running"
    done
}

case "${1:-help}" in
    add)    cmd_add "${2:-}" ;;
    remove) cmd_remove "${2:-}" ;;
    list)   cmd_list ;;
    *)      echo "Usage: $0 {add|remove|list} [name]" ;;
esac

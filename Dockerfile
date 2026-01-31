FROM rust:1.88 AS builder
WORKDIR /src
COPY . .
RUN cargo build --release --bin sq

FROM debian:bookworm-slim
WORKDIR /exo
# Copy artefact stripped of debug symbols
COPY --from=builder /src/target/release/sq ./sq
RUN mkdir -p /exo/data
EXPOSE 1337

# Auth and tenant isolation via environment variables:
#   SQ_KEY      - API key (e.g., pmb-v1-xxx). Omit to disable auth.
#   SQ_DATA_DIR - Tenant data directory (default: /exo/data)
#   SQ_PORT     - Port to listen on (default: 1337)
#
# Example: docker run -e SQ_KEY=pmb-v1-abc123 -p 1337:1337 wbic16/sq:0.5.0
ENV SQ_PORT=1337
ENV SQ_DATA_DIR=/exo/data

CMD sh -c './sq host ${SQ_PORT} $([ -n "$SQ_KEY" ] && echo "--key $SQ_KEY") --data-dir ${SQ_DATA_DIR}'
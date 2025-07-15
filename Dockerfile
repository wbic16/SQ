FROM rust:1.88 AS builder
WORKDIR /src
COPY . .
RUN cargo build --release --bin sq

FROM debian:bookworm-slim
WORKDIR /exo
# Copy artefact stripped of debug symbols
COPY --from=builder /src/target/release/sq ./sq
COPY ./CYOA.phext ./CYOA.phext
EXPOSE 1337
CMD ["./sq", "host", "1337"]
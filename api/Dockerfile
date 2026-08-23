FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY api/Cargo.toml api/Cargo.toml
RUN mkdir -p api/src && echo "fn main() {}" > api/src/main.rs
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY api api
RUN cargo build --release -p api

FROM debian:trixie-slim AS runtime
# Base bumped bookworm -> trixie (2026-08-23, incidental to step 7.3): `cargo-chef:latest-rust-1`
# is a rolling tag that has drifted to Debian trixie upstream since this Dockerfile was first
# written (confirmed via `docker run --rm lukemathwalker/cargo-chef:latest-rust-1 cat
# /etc/os-release`) -- copying a trixie-built (glibc 2.39) binary onto a bookworm-slim (glibc 2.36)
# runtime fails at startup with "GLIBC_2.39 not found". Runtime now matches the builder's actual
# base rather than a base pinned at Phase 0.1 that upstream silently moved past. Pin BOTH stages to
# a specific Debian release together if this drift becomes a recurring problem.
#
# pg_dump/pg_restore, version-matched to the postgres:17 service (a version mismatch between client
# and server tools is a real hazard for `-Fc` custom-format dumps) — trixie's own default
# `postgresql-client` package tracks whatever's current there, so this pulls the PGDG apt repo for
# v17 specifically rather than trusting the distro default to stay pinned.
RUN apt-get update && apt-get install -y --no-install-recommends wget ca-certificates gnupg lsb-release \
    && wget --quiet -O - https://www.postgresql.org/media/keys/ACCC4CF8.asc | gpg --dearmor -o /usr/share/keyrings/pgdg.gpg \
    && echo "deb [signed-by=/usr/share/keyrings/pgdg.gpg] http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" > /etc/apt/sources.list.d/pgdg.list \
    && apt-get update && apt-get install -y --no-install-recommends postgresql-client-17 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/api /usr/local/bin/api
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/api"]

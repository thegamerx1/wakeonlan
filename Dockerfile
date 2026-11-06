FROM --platform=$BUILDPLATFORM node:24-alpine AS frontend
ARG PNPM_VERSION=12.18.0
RUN npm install --global pnpm@${PNPM_VERSION}
WORKDIR /app

COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml web/.npmrc ./
RUN pnpm install --frozen-lockfile
COPY web/. .
RUN pnpm build

FROM rust:1.85-bookworm AS backend
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml ./backend/Cargo.toml
COPY backend/src ./backend/src
COPY agent/Cargo.toml ./agent/Cargo.toml
COPY agent/src ./agent/src

RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/app/target cargo build --release -p backend && cp ./target/release/backend /app/wakeonlan-backend

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=frontend /app/build /app/public
COPY --from=backend /app/wakeonlan-backend /app/backend

ENV RUST_LOG=info
ENV APP_PORT=80
ENV APP_HOST=0.0.0.0
ENTRYPOINT ["/app/backend"]

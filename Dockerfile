FROM --platform=$BUILDPLATFORM node:18-alpine AS frontend
RUN npm i -g pnpm
RUN mkdir -p /app/
WORKDIR /app

COPY web/package*.json ./
COPY web/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/. .
RUN pnpm build


FROM --platform=$BUILDPLATFORM rust:1.80 AS backend
WORKDIR /app

COPY ./Cargo.lock ./backend/Cargo.toml .
COPY ./backend/src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/app/target cargo build --release && cp ./target/release/backend .

FROM debian:bookworm-slim
WORKDIR /app

COPY --from=frontend /app/build /app/public
COPY --from=backend /app/backend /app/

ENV RUST_LOG=info
ENV APP_PORT=80
ENV APP_HOST=0.0.0.0
ENTRYPOINT [ "./backend" ]
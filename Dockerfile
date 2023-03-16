FROM node:16-alpine AS base
RUN npm i -g pnpm

FROM base AS build
RUN mkdir -p /app/
WORKDIR /app

COPY web/package*.json ./
COPY web/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/. .
RUN pnpm build

FROM base AS webserver
ENV NODE_ENV=production
RUN mkdir -p /app/
WORKDIR /app

COPY server/package*.json ./
COPY server/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY server/. .
RUN pnpm build
COPY --from=build /app/build /app/public
# Ugly fix that works..
RUN mv dist/* .

# TODO: Volume data folder

EXPOSE 80
CMD [ "node", "server.js" ]
# Installation Guide

> ⚠️ *Official images are not yet published. This document describes the target installation methods; they will be available from the 0.1.0 release onward.*

## Prerequisites

- **Docker** and **Docker Compose** (recommended)
- Running instances of at least one of: **Seerr**, **Radarr**, **Sonarr**, **Prowlarr**
- Access to the APIs of the applications you want to integrate (API keys)

Supported platforms: **Linux AMD64** and **Linux ARM64**.

## Option A: Official Docker image (recommended)

Official images are published to GitHub Container Registry:

```
ghcr.io/kiritogxe/gxe-search-queue
```

```sh
# Pull the latest tagged release
docker pull ghcr.io/kiritogxe/gxe-search-queue:latest

# Run with a persistent data volume
docker run -d \
  --name gxe-search-queue \
  --restart unless-stopped \
  -p 8080:8080 \
  -v gxe-search-queue-data:/app/data \
  ghcr.io/kiritogxe/gxe-search-queue:latest
```

Configure via environment variables (copy `.env.example` as a starting point):

```sh
docker run -d \
  --name gxe-search-queue \
  --env-file ./gxe-search-queue.env \
  -p 8080:8080 \
  -v gxe-search-queue-data:/app/data \
  ghcr.io/kiritogxe/gxe-search-queue:latest
```

Pinned versions are preferred for production (e.g. `:0.1.0` instead of `:latest`).

## Option B: Docker Compose (local development / evaluation)

1. Clone the repository:
   ```sh
   git clone https://github.com/KiritoGXE/gxe-search-queue.git
   cd gxe-search-queue
   ```

2. Copy the sample environment file and adjust:
   ```sh
   cp .env.example .env
   ```

3. Launch the stack:
   ```sh
   docker compose up -d
   ```

4. Verify:
   - API: `http://localhost:8080`
   - Dashboard: `http://localhost:8081`

## Option C: Self-build from source

Build and run the image directly from the repository (see [Development guide](development.md) for the toolchain):

```sh
docker build -t gxe-search-queue:dev .
docker run --rm gxe-search-queue:dev --help
```

## Configuration reference

See [configuration.md](configuration.md) for the full list of environment variables and configuration options.

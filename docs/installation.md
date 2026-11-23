# Installation Guide

## Prerequisites

- **Rust** 1.75+ (install via `rustup` if building from source)
- **Docker & Docker Compose** (for containerized local development)
- **PostgreSQL** 14+
- **Redis** 7+

## Option A: Docker Compose (Recommended for local evaluation)

1. Clone the repository:
   ```sh
   git clone https://github.com/your-org/gxe-search-queue.git
   cd gxe-search-queue
   ```

2. Copy the sample environment file:
   ```sh
   cp .env.example .env
   ```

3. Launch the container stack:
   ```sh
   docker compose up -d
   ```

4. Verify services:
   - REST API: `http://localhost:8080/health`
   - Dashboard: `http://localhost:8081`

## Option B: Native Build (For development)

1. Ensure Postgres and Redis are running (e.g. via `docker compose up -d postgres redis`).

2. Build and run via Cargo:
   ```sh
   cp .env.example .env
   cargo run -- serve
   ```

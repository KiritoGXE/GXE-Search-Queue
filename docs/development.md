# Development Guide

## Workflow

1. Setup development dependencies:
   ```sh
   cp .env.example .env
   docker compose up -d postgres redis
   ```

2. Run the application locally:
   ```sh
   cargo run
   ```

3. Run tests:
   ```sh
   # All tests
   cargo test

   # Specific integration tests
   cargo test --test integration
   ```

4. Code formatting and linting:
   ```sh
   cargo fmt
   cargo clippy -- -D warnings
   ```

## Folder Structure Guidelines

- Place queue primitives, state machines, and task models in `src/core/`.
- Place scheduling mechanisms in `src/scheduler/`.
- Add connectors or third-party web crawlers into `src/integrations/`.
- Add database models, queries, and migrations under `src/database/`.
- REST API route handlers belong in `src/api/`.
- Web UI assets / handlers belong in `src/dashboard/`.
- Webhooks and email sending mechanisms live in `src/notifications/`.

## Fixtures & Test Data

Store reusable JSON or SQL payloads in `tests/fixtures/`.

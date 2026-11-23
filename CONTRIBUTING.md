# Contributing to GXE-Search-Queue

Thanks for your interest in contributing! Here is how you can help.

## Ways to contribute

- Report bugs and request features via [GitHub Issues](https://github.com/) using the provided templates.
- Improve documentation in `docs/`.
- Submit code changes via pull requests.
- Review open pull requests.

## Getting started

1. Read the [development guide](docs/development.md) and [architecture](docs/architecture.md).
2. Set up your local environment following [docs/installation.md](docs/installation.md).
3. Find an issue to work on, or create one describing the change you plan to make.

## Code of conduct

This project follows the [Code of Conduct](CODE_OF_CONDUCT.md). All participants are expected to uphold it.

## Branching and workflow

- Use short, descriptive branch names (e.g. `feat/retry-policy`, `fix/scheduler-dst`).
- Keep pull requests focused on a single change.
- Write a clear pull request description using the provided [template](.github/PULL_REQUEST_TEMPLATE.md).
- Rebase your branch on the latest `main` before requesting review.

## Style and quality gates

- Format code with `cargo fmt` (configuration lives in `rustfmt.toml`).
- Keep `cargo clippy -- -D warnings` clean.
- Run the full test suite with `cargo test` before pushing.
- Add tests for new functionality (`tests/unit`, `tests/integration`, or inline `#[cfg(test)]`).

## Commit messages

Use conventional commit style:

```
feat(queue): add retry with exponential backoff
fix(scheduler): respect timezone for cron jobs
docs(architecture): describe delivery semantics
```

## Licensing

By contributing you agree that your contributions are licensed under the same
[MIT License](LICENSE) as this project.

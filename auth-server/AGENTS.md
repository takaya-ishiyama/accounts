# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2021 Cloudflare Workers auth service. The Worker entry point is `src/lib.rs`, which wires Axum routes, queue handling, and security headers.

- `src/adapter/`: HTTP API handlers and boundary code.
- `src/application/`: use cases, command/query flows, projections, and HTTP helpers.
- `src/domain/`: models, auth rules, events, and pure business logic.
- `src/infrastructure/`: Cloudflare bindings, repositories, providers, tokens, sessions, and environment adapters.
- `db/schema.sql` and `db/migrations/`: Atlas-managed D1 schema and migrations.
- `docs/`, `README.md`, and `USAGE.md`: design notes and operational usage.

## Build, Test, and Development Commands

- `mise run dev`: starts Wrangler development on port `8000`.
- `cargo test`: runs Rust unit tests.
- `cargo fmt --check`: checks Rust formatting.
- `cargo clippy --all-targets --all-features`: runs Rust lints.
- `mise run migrate-local`: applies the initial D1 migration locally.
- `mise run migrate-diff`: generates Atlas migration diffs from `db/schema.sql`.
- `MISE_ENV=staging mise run migrate-staging` / `mise run deploy-staging`: applies and deploys staging.
- `MISE_ENV=production mise run migrate-production` / `mise run deploy-production`: applies and deploys production.

Wrangler build configuration lives in `wrangler.toml`; it installs `worker-build@^0.8` and builds release WASM.

## Coding Style & Naming Conventions

Use standard Rust formatting with `rustfmt`; keep imports tidy and module boundaries explicit. Follow existing snake_case file names such as `refresh_token_store.rs` and `auth_google_callback.rs`. Domain types use `PascalCase`, functions and variables use `snake_case`, and constants use `SCREAMING_SNAKE_CASE` when needed. Keep domain code free of Cloudflare APIs; place Worker, D1, KV, queue, and provider integrations in `infrastructure` or `adapter`.

## Testing Guidelines

Unit tests live next to implementation files in `#[cfg(test)] mod tests` blocks. Add tests for domain rules, parsers, validation, and use case branching when behavior changes. Prefer deterministic tests with local fixtures over network calls. Run `cargo test` before submitting changes; add focused tests near the changed module unless Worker bindings require integration coverage.

## Commit & Pull Request Guidelines

Recent history uses short imperative subjects such as `fix state`, `refactor`, and `usage`. Keep commits small and focused. Pull requests should include a summary, test results (`cargo test`, formatting/lint checks), migration notes for `db/` changes, and deployment or secret updates when `wrangler.toml` or environment behavior changes.

## Security & Configuration Tips

Do not commit secrets from `.env.local`, `.env.staging`, `.env.production`, or Wrangler secret values. Review OAuth redirect URIs, JWT settings, D1 database names, KV namespace IDs, Durable Objects, and queue bindings in `wrangler.toml` when changing environment-specific behavior.

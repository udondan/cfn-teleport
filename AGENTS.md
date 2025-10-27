# AGENTS.md

Quick reference for AI coding agents working on cfn-teleport.

## Build/Test/Lint Commands

- `cargo build` - Build the project
- `cargo test --all` - Run all unit tests
- `cargo test <test_name>` - Run a specific test by name
- `cargo check` - Quick compilation check
- `make test` - Full test suite (unit + CDK integration tests)
- `make lint` - Run formatting and clippy checks (`cargo fmt -- --check` + `cargo clippy -- -D warnings`)

## Code Style

- **Edition**: Rust 2021
- **Formatting**: Use `cargo fmt` (enforced by CI)
- **Lints**: Clippy with `-D warnings` (all warnings are errors)
- **Imports**: Group by std, external crates, then local modules (see main.rs:1-11)
- **Error Handling**: Use `Result<T, Box<dyn Error>>` for main functions, propagate errors with `?`
- **Naming**: snake_case for functions/variables, PascalCase for types, SCREAMING_SNAKE for constants
- **Async**: Use `#[tokio::main]` for entry point, `async fn` for AWS SDK calls
- **Types**: Prefer explicit types for function signatures, use type inference in function bodies

## Git Workflow

- Conventional commits required (feat:, fix:, refactor:, etc.)

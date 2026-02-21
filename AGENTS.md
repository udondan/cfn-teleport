# AGENTS.md

Quick reference for AI coding agents working on cfn-teleport.

## Project Overview

**cfn-teleport** is a Rust CLI tool that moves CloudFormation resources between stacks using AWS SDK for Rust. It features interactive prompts (dialoguer), async AWS operations (tokio), and integrates with CDK-based integration tests.

## Build/Test/Lint Commands

### Building

- `make build` - Build debug binary
- `make release` - Build optimized release binary
- `cargo check` - Fast compilation check without producing binary
- `cargo install --path .` - Install binary locally from source

### Testing

- `cargo test --all` - Run all unit tests (if any exist)
- `cargo test <test_name>` - Run a specific test by name (e.g., `cargo test split_ids`)
- `cargo test <module>::<test>` - Run specific test in module
- `make test` - Full test suite:
  - Runs `cargo check`
  - Runs `cargo test --all`
  - Deploys CDK test stacks (requires AWS credentials)
  - Runs integration tests with actual AWS resources

### Linting & Formatting

- `cargo fmt` - Auto-format code
- `cargo fmt -- --check` - Check formatting without modifying files
- `cargo clippy` - Run linter with default warnings
- `cargo clippy -- -D warnings` - Run linter treating all warnings as errors (CI mode)
- `make lint` - Run both fmt check and clippy with `-D warnings` (CI equivalent)

### Running

- `cargo run` - Run the program in debug mode
- `cargo run -- --help` - Run with arguments (e.g., help)
- `cargo run -- --source Stack1 --target Stack2` - Run with specific options
- `make run` - Same as `cargo run`

### CDK Integration Tests

- `cd test/cdk && make install` - Install Node dependencies for test stacks
- `cd test/cdk && make diff` - Check CDK stack changes
- `cd test/cdk && make deploy` - Deploy test stacks to AWS
- `cd test/cdk && make DESTROY` - Destroy all test stacks
- `make test-clean-all` - Clean up all test resources by tags (S3, DynamoDB, EC2, etc.)
- `make test-reset` - Full reset: destroy stacks + clean all tagged resources

## Code Style & Conventions

### Rust Edition & Tooling

- **Edition**: Rust 2021
- **Formatting**: Use `cargo fmt` (enforced by CI via `cargo fmt -- --check`)
- **Lints**: Clippy with `-D warnings` - **all warnings are treated as errors** in CI
- **Minimum Supported Rust Version (MSRV)**: Test against stable and nightly toolchains

### Import Organization

Group imports in this order (see `main.rs:1-11`):

1. External crate imports (alphabetical)
2. Local module declarations (`mod`)
3. Local module imports (`use crate::`)

Example:

```rust
use aws_config::BehaviorVersion;
use aws_sdk_cloudformation as cloudformation;
use clap::Parser;
use dialoguer::{Confirm, Input, MultiSelect};
use std::collections::HashMap;
use std::error::Error;
use std::io;
mod spinner;
mod supported_resource_types;
```

### Naming Conventions

- **Functions & variables**: `snake_case` (e.g., `get_stacks`, `source_stack`)
- **Types & structs**: `PascalCase` (e.g., `Args`, `Spin`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEMO`, `SUPPORTED_RESOURCE_TYPES`)
- **Lifetimes**: Single lowercase letter (e.g., `'a`)

### Error Handling

- **Main & fallible functions**: Return `Result<T, Box<dyn Error>>` for flexibility
- **AWS SDK operations**: Return `Result<T, cloudformation::Error>` for specific AWS errors
- **Error propagation**: Use `?` operator for concise error handling
- **Custom errors**: Convert to `Box<dyn Error>` with `.into()` or string conversion
- **User-facing errors**: Use `eprintln!` for error messages with helpful context (see `main.rs:63-73`)
- **Process exit**: Use `process::exit(1)` for fatal errors after printing user-friendly messages

Example:

```rust
async fn get_stacks(
    client: &cloudformation::Client,
) -> Result<Vec<cloudformation::types::StackSummary>, cloudformation::Error> {
    let resp = client.list_stacks().send().await?;
    Ok(resp.stack_summaries.unwrap_or_default())
}
```

### Type Annotations

- **Function signatures**: Always use explicit types for parameters and return values
- **Function bodies**: Use type inference (`let var = ...`) for local variables when clear
- **Complex types**: Consider type aliases for readability

### Async/Await Patterns

- **Entry point**: Use `#[tokio::main]` on `async fn main()`
- **AWS operations**: All AWS SDK calls are `async` - use `.await?` pattern
- **Dependencies**: `tokio = { version = "1.37.0", features = ["full"] }`
- **Threading**: Use `std::thread::sleep` for blocking waits (see `main.rs:895`)

### Documentation

- Use `///` for public API documentation comments
- Use `//` for inline explanatory comments
- Use `//!` for module-level documentation
- CLI arg documentation: Use `#[arg(...)]` doc attributes (see `main.rs:19-33`)

### Code Organization

- **Modules**: Separate concerns into modules (`spinner.rs`, `supported_resource_types.rs`)
- **Function size**: Keep functions focused; extract helpers (e.g., `split_ids`, `user_confirm`)
- **Constants**: Extract magic values and large static data to module-level constants

### User Interaction Patterns

- **Prompts**: Use `dialoguer` crate for interactive prompts (`Select`, `MultiSelect`, `Confirm`, `Input`)
- **Spinners**: Use custom `Spin` wrapper from `spinner.rs` for progress indication
- **Output**: Use `println!` for normal output, `eprintln!` for errors
- **TTY detection**: Check `atty::is(Stream::Stdout)` for interactive vs. piped output

### Dependencies & Features

- **AWS SDK**: Use `aws-config` and `aws-sdk-cloudformation` with explicit `BehaviorVersion`
- **CLI parsing**: Use `clap` with derive macros (`#[derive(Parser)]`)
- **JSON**: Use `serde_json::Value` for template manipulation
- **UUIDs**: Use `uuid` crate with `v4` and `fast-rng` features

## Git Workflow

### Commit Messages

- **Required format**: Conventional Commits (enforced by CI)
- **Types**: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`, etc.
- **Examples**:
  - `feat: add support for cross-region resource migration`
  - `fix: handle missing resource identifiers gracefully`
  - `refactor: extract template validation logic`
  - `docs: update README with new command examples`

### Pull Request Process

- PRs must have conventional commit titles (enforced by `.github/workflows/pr-conventional-pr-title.yml`)
- CI runs on all PRs against `main` branch
- Tests run on both `stable` and `nightly` Rust toolchains
- Must pass: formatting check, clippy, unit tests, integration tests

### Release Process

- Uses `release-please` for automated releases
- Version bumps based on conventional commit messages
- Automatically generates `CHANGELOG.md`
- Publishes to crates.io on release

## Testing Strategy

### Unit Tests

- Currently minimal unit test coverage (focus on integration tests)
- Add unit tests in function/module with `#[cfg(test)]` and `#[test]` attributes

### Integration Tests

- Uses AWS CDK to deploy real CloudFormation stacks (`test/cdk/`)
- Tests actual resource migration between `CfnTeleportTest1` and `CfnTeleportTest2` stacks
- Validates resources: S3 buckets, DynamoDB tables, EC2 instances, security groups, IAM roles
- **Requires**: AWS credentials in CI secrets (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
- **Region**: Tests run in `us-east-1`

### CI Pipeline

1. **Path filtering**: Only runs tests if `src/`, `test/`, or `Cargo.*` files change
2. **Matrix testing**: Runs on both `stable` and `nightly` Rust
3. **Lint checks**: Only on `stable` toolchain
4. **Integration tests**: Only on `stable` toolchain (requires AWS credentials)
5. **Cleanup**: Automatically destroys test resources after tests

## Common Development Tasks

### Adding a New Feature

1. Create a feature branch: `git checkout -b feat/my-feature`
2. Make changes following code style guidelines
3. Run `cargo fmt` before committing
4. Run `make lint` to check for clippy warnings
5. Test locally: `cargo run -- --help` or specific test scenarios
6. Commit with conventional format: `git commit -m "feat: description"`
7. Open PR with conventional title

### Debugging AWS Integration

- Enable AWS SDK logging: `export RUST_LOG=aws_config=debug,aws_sdk_cloudformation=debug`
- Use `--yes` flag to skip interactive prompts during testing
- Check CloudFormation console for stack events during failures

### Working with Templates

- Templates are manipulated as `serde_json::Value` objects
- Use `serde_json::to_string_pretty()` for readable output
- Validate templates with `validate_template()` before applying changes

## Architecture Notes

- **Single binary**: All code in `src/main.rs` with small helper modules
- **State management**: Uses `HashMap` for resource ID mappings (`new_logical_ids_map`)
- **Async flow**: Sequential AWS operations with `await`, no parallelism needed
- **Error recovery**: Gracefully handles AWS errors with user-friendly messages
- **Interactive mode**: Falls back to prompts when CLI args not provided
- **Change sets**: Uses CloudFormation import/export change sets for resource moves

## Reference Files

- `src/main.rs:1-11` - Import organization example
- `src/main.rs:37-87` - Error handling pattern for AWS credentials
- `src/main.rs:312-319` - Simple utility function example
- `src/spinner.rs` - Custom progress indicator wrapper
- `Cargo.toml` - Dependencies and project metadata
- `.github/workflows/pr-test.yml` - CI pipeline configuration

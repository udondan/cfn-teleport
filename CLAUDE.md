# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

cfn-teleport is a Rust CLI tool that moves CloudFormation resources between AWS stacks. The core functionality involves:
1. Removing resources from source stack with DeletionPolicy=Retain
2. Importing retained resources into target stack
3. Cleaning up the target stack by removing temporary DeletionPolicy attributes

## Key Development Commands

### Building and Running
- `cargo build` - Build the project
- `cargo run` - Run the application locally
- `cargo build --release` - Build optimized release version

### Testing
- `cargo check` - Quick compilation check
- `cargo test --all` - Run unit tests
- `make test` - Full test suite including CDK integration tests

### Code Quality
- `cargo fmt -- --check` - Check formatting
- `cargo clippy -- -D warnings` - Run linter with strict warnings
- `make lint` - Run both formatting and clippy checks

## Architecture

### Core Components
- `main.rs` - Entry point with CLI parsing and orchestration
- `spinner.rs` - Terminal spinner for long-running operations
- `supported_resource_types.rs` - Whitelist of CloudFormation resource types

### Key Functions in main.rs
- Stack operations: `get_stacks()`, `get_resources()`, `get_template()`
- Template manipulation: `retain_resources()`, `remove_resources()`, `add_resources()`
- CloudFormation operations: `update_stack()`, `create_changeset()`, `execute_changeset()`
- UI/UX: `select_stack()`, `select_resources()`, `format_resources()`

### Resource Movement Flow
1. Select source/target stacks and resources
2. Update source stack to retain selected resources
3. Remove resources from source stack (they remain due to retain policy)
4. Import resources into target stack via changeset
5. Remove temporary DeletionPolicy attributes from target stack

## Dependencies
- AWS SDK for CloudFormation operations
- clap for CLI parsing
- dialoguer for interactive prompts
- serde_json for template manipulation
- tokio for async operations

## Testing Structure
- Unit tests in `src/` files
- Integration tests in `test/cdk/` using AWS CDK
- Test cleanup commands in Makefile for AWS resource cleanup
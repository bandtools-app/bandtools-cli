# AGENTS.md

## Project Overview

This repository contains `bt`, a Rust command line interface for the BandTools REST API.

The CLI is organised into resource groups and commands. Keep the command surface consistent with the existing `clap`-based structure in `src/cli.rs`, and route command execution through `src/commands.rs`.

## Repository Rules

- Use UK spelling in prose, comments, test names, and documentation where practical.
- Do not add generated API specifications or private API reference files to the repository.
- Do not include real non-production BandTools environment URLs in source, tests, docs, commits, or examples.
- Do not commit secrets, API tokens, local config files, or generated build output.
- Keep `Cargo.lock` committed.
- Keep the executable name as `bt`.

## Code Structure

- `src/cli.rs`: command-line interface definitions and argument parsing.
- `src/commands.rs`: command dispatch, request body shaping, and output handling.
- `src/api.rs`: HTTP client, request construction, response decoding, and upload helpers.
- `src/config.rs`: config file loading, environment variable precedence, and API URL handling.
- `src/output.rs`: JSON output formatting.
- `tests/`: integration tests for CLI help, config, and HTTP request construction.

Prefer extending these existing modules over adding new abstractions.

## Implementation Guidelines

- Preserve token precedence: command-line option, environment variable, then config file.
- Preserve support for overriding the API base URL without documenting real private URLs.
- Keep API responses as JSON output unless a command already has a different established shape.
- For JSON request bodies, continue supporting `--data` and `--data-file`.
- For wrapped API resources, continue accepting unwrapped input where existing commands already do so.
- Use `anyhow` for application-level errors unless a more specific local error type already exists.
- Use blocking `reqwest`, matching the current client design.
- Avoid broad refactors unless they directly support the requested change.

## Testing and Verification

Run these before considering work complete:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Use `cargo llvm-cov --all-targets --workspace` when changing test coverage or CI coverage reporting.

Add or update tests when changing:

- CLI parsing or help output.
- Config precedence or environment variable handling.
- Request paths, query parameters, headers, methods, or JSON bodies.
- Error handling or response rendering.
- File upload behaviour.

Use `wiremock` for HTTP request tests and `assert_cmd` for binary-level CLI tests.

## Git and History Hygiene

- Keep commits focused and reviewable.
- Do not rewrite published history unless explicitly requested.
- If history is rewritten to remove sensitive material, verify with history-wide searches before force-pushing.

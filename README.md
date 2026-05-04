# BandTools CLI

`bt` is a Rust command line interface for the BandTools REST API.

The command layout is organised into resource groups and commands:

```sh
bt subscribers list
bt subscribers add --email-address fan@example.com
bt newsletters list --status draft
bt account get
```

## Installation

Build from this repository:

```sh
cargo build --release
```

The executable is named `bt` and will be written to `target/release/bt`.

## Authentication and Configuration

API tokens can be supplied in three ways. Later entries in this list are lower
priority:

1. `--api-token TOKEN`
2. `BANDTOOLS_API_TOKEN` or `BT_API_TOKEN`
3. `api_token` in the config file

The default config file is:

```text
~/.config/bandtools/config.toml
```

Use `--config PATH`, `BANDTOOLS_CONFIG`, or `BT_CONFIG` to choose another file.

Config helpers:

```sh
bt config path
bt config set-token TOKEN
bt config set-api-url http://localhost:3000/api/v1
bt config show
bt config unset api-token
```

The production API URL defaults to `https://bandtools.app/api/v1`. A hidden
global `--api-url URL` option and `BANDTOOLS_API_URL` or `BT_API_URL`
environment variables are available for non-production test environments.

## JSON Bodies

Commands that send structured request bodies accept either inline JSON or a
file:

```sh
bt account update --data '{"name":"New Artist Name"}'
bt newsletters create --data-file newsletter.json
```

For resource-specific wrappers such as `account`, `settings`, `theme`, and
`automatic_newsletter`, `bt` accepts either the wrapped API shape or the
unwrapped object and wraps it automatically.

## Output

By default, `bt` renders API responses as terminal-friendly tables and panels.
Use `--json` when you need raw compact JSON for scripts or piping:

```sh
bt subscribers list
bt --json subscribers list
```

## Command Help

Every command group and command supports built-in help:

```sh
bt --help
bt subscribers --help
bt subscribers list --help
bt newsletters schedule --help
```

## Testing

Run the full test suite:

```sh
cargo test
```

The tests include parser/help coverage, config behaviour, request construction,
authorisation header precedence, and mock HTTP API calls.

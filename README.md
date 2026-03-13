# gateflow.net

`gateflow.net` is a multi-service gateway workspace with three main components:

- `gateflow` (Rust): gateway core (`HTTP dataplane`, `Admin gRPC`, `UDP health ingest`)
- `healthd` (Go): health checker and UDP reporter
- `client` (Go): admin CLI for gateway operations

## Repository Layout

- `gateflow/`: Rust service, DB migrations, gateway docs
- `healthd/`: Go health daemon
- `client/`: Go CLI

## Quick Start

1. Start dependencies (mainly Postgres) and set service configs.
2. Run `gateflow`:
```bash
cd gateflow
cargo run .
```
3. Run `client` (example login):
```bash
cd client
APP_HOST=127.0.0.1:50051 go run . login --username <user> --password <pass>
```
4. Run `healthd`:
```bash
cd healthd
go run .
```

## Integration Order (Recommended)

Run the full linkage in this order:

1. Start `gateflow` first (it provides Admin gRPC + UDP ingest).
2. Use `client` to `login` and obtain a `sessionToken`.
3. Put the token into `healthd/healthd.yaml` as `gateway_session_token`.
4. Start `healthd` and confirm reports are accepted by `gateflow`.
5. Use `client app list/show` to confirm control-plane and health linkage.

Quick smoke sequence:

```bash
# 1) gateflow
cd gateflow && cargo run .

# 2) client login
cd ../client
APP_HOST=127.0.0.1:50051 go run . login --username <user> --password <pass>

# 3) set healthd token (edit healthd/healthd.yaml), then run
cd ../healthd
go run .
```

## Test Commands

- Gateflow:
```bash
cd gateflow
cargo test
```

- healthd:
```bash
cd healthd
GOCACHE=/tmp/.gocache-healthd go test ./...
```

- client:
```bash
cd client
GOCACHE=/tmp/.gocache-client go test ./...
```

## Docs

- Gateway overview and operations:
  - `gateflow/docs/README.md`
  - `gateflow/docs/OPERATIONS.md`
- Git commit convention:
  - `gateflow/docs/git.md`

## Commit Convention (Summary)

Use:

```text
<type>(<project>): <subject>
```

- `project`: `client` | `healthd` | `gateflow`
- Do not mix changes from different projects in one commit.

## License

This repository is licensed under Apache License 2.0.
See `LICENSE` and `NOTICE` for details.

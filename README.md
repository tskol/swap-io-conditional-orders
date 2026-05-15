# ORDO

ORDO is an Anchor-based Solana program for conditional order execution.

## Program IDs

- Mainnet-beta: `EfsKVSxQxwoR9NpZfjgWuuwpbbF14BQgrh3rt4kAs181`
- Devnet: `3kcDAfBY5Q5cKJMwWKX3DRqGg3JwQ7Tntkh8faYF5yA5`

## Workspace Layout

- `programs/ordo`: on-chain program
- `scripts`: operational TypeScript helpers
- `tests`: integration and flow coverage

## Install

```bash
npm install
```

## Build

Preferred local build:

```bash
npm run build:program
```

TypeScript check:

```bash
npm run check:ts
```

Rust checks:

```bash
cargo clippy -p ordo --lib --tests -- -W clippy::cognitive_complexity -W clippy::too_many_lines
cargo test -p ordo --tests
```

Coverage:

```bash
cargo llvm-cov -p ordo --tests --lib --summary-only
```

## Local Test Flow

```bash
surfpool start
npm run build:program
anchor deploy --provider.cluster localnet
anchor test --skip-local-validator --skip-deploy
```

## Scripts

Anchor script aliases are defined in `Anchor.toml`:

- `BootstrapGlobalConfig`
- `ConfigureOraclePool`
- `ConfigureVault`
- `SubmitOrder`
- `ExecuteOrder`
- `ExitOrder`
- `UpdateCounterparty`

## Environment

See [.env.example](./.env.example) for script variables.

## Security

See [SECURITY.md](./SECURITY.md).

## Acknowledgements

This codebase was developed from earlier order-execution work in the Solana ecosystem.

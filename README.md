# LIMO (Liquidity Integration & Matching Orders) — Solana Program

This directory contains the **LIMO** on-chain program (Anchor/Solana).

## Attribution

This contract was built **based on** the upstream `kamino/limo` program by Kamino Finance.

- Upstream reference: [`kamino-finance/limo`](https://github.com/Kamino-Finance/limo) (and related Kamino Finance repositories)

## Program ID

- **Program ID**: `82qR3ftARSeXPBg9wnVrsLsAepfvimBUCPdNuiV9unhV` (see `programs/limo/src/lib.rs`)
- **Deployments**:
  - **Mainnet-beta**: deployed at the same Program ID
  - **Devnet**: deployed at the same Program ID
  - **Localnet / Surfpool localnet**: deploy the program to your local cluster (e.g. `anchor deploy`) before running tests

## What it does (high level)

LIMO is an order-based swap/matching program that lets a **maker** create an order to swap `input_mint` for `output_mint`, and a **taker** fill (take) the order under configurable constraints. The program maintains shared configuration (global config), token vaults, and optional oracle pools used for price / staleness checks.

## Main on-chain instructions

From `programs/limo/src/lib.rs` / `target/idl/limo.json`:

- **`initialize_global_config`**: creates/initializes the global configuration account (admin + PDA authority).
- **`initialize_oracle_pool(feed_id)`**: registers an oracle feed for a given mint (Pyth receiver integration is used in this codebase).
- **`initialize_vault`**: initializes vault(s) for a mint used by orders.
- **`create_order(input_amount, output_amount, order_type, tp_output_amount, sl_output_amount)`**: creates an order (supports optional TP/SL child orders when enabled/configured).
- **`update_order(mode, value)`**: updates specific order fields (see `UpdateOrderMode` in the IDL).
- **`take_order(input_amount, min_output_amount, tip_amount_permissionless_taking)`**: fills an order partially or fully, subject to global constraints and minimum output checks.
- **`close_order_and_claim_tip`**: closes an order (and optional child orders) and claims any associated tip.
- **`update_global_config(mode, value)`** and **`update_global_config_admin`**: updates global settings / cached admin authority.
- **`withdraw_host_tip`**: withdraws host tip accumulated in the PDA authority.
- **`log_user_swap_balances_*`** and **`assert_user_swap_balances_*`**: helper instructions for recording and asserting balance deltas around swap flows.

## Order types

See `OrderType` in the IDL:

- `Vanilla`
- `LimitParent`
- `LimitTP`
- `LimitSL`

## Safety switches / constraints

The program enforces global constraints via access controls, including (names per code/IDL):

- **Emergency mode** (blocks certain operations)
- **Block new orders**
- **Block order taking**
- (Flash take order hooks exist in code but are commented out in `lib.rs` in this repo)

## Repo layout (program)

- `programs/limo/src/lib.rs`: program entrypoints (Anchor instructions) + error codes
- `programs/limo/src/handlers/`: instruction handlers
- `programs/limo/src/state/`: account state definitions (e.g., `GlobalConfig`, `Order`, `OraclePoolsState`)
- `programs/limo/src/utils/`: constraints, constants, helper logic

## Build & test (local)

### Prerequisites

- **Rust** toolchain compatible with Solana + Anchor
- **Anchor** `0.29.0` (see `Anchor.toml`)
- **Node/Yarn** (this repo uses `yarn` and `ts-node`)
- **Surfpool** installed (required for running tests in this repo)

### Install JS deps

From repository root:

```bash
yarn install
```

### Build the program

From repository root:

```bash
anchor build
```

### Run tests

From repository root:

```bash
# 1) Start Surfpool localnet
surfpool start

# 2) Deploy the programs to the Surfpool localnet
anchor build
anchor deploy --provider.cluster localnet

# 3) Run tests against Surfpool localnet
anchor test --skip-local-validator --skip-deploy
```

Notes:

- Ensure your Anchor provider is pointing at the Surfpool localnet RPC (e.g. via `Anchor.toml` or `ANCHOR_PROVIDER_URL`).
- When finished, stop the localnet if needed (for example `surfpool stop`).

## Useful scripts

`Anchor.toml` exposes helper scripts that call TypeScript under `./scripts/`:

```bash
anchor run initializeGlobalConfig
anchor run InitializeOraclePool
anchor run InitializeVault
anchor run CreateOrder
anchor run TakeOrder
anchor run CloseOrder
```

Scripts live in:

- `scripts/InitializeGlobalConfig.ts`
- `scripts/InitializeOraclePool.ts`
- `scripts/InitializeVault.ts`
- `scripts/CreateOrder.ts`
- `scripts/TakeOrder.ts`
- `scripts/CloseOrder.ts`

## IDL

The Anchor IDL for this program is generated into `target/idl/limo.json`.

```bash
anchor build
ls -la target/idl/
```

## Security notes

- This codebase includes `solana-security-txt` metadata (see `programs/limo/src/lib.rs`).


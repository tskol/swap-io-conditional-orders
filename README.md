# LIMO (Liquidity Integration & Matching Orders) — Solana Program

This directory contains the **LIMO** on-chain program (Anchor/Solana).

## Attribution

This contract was built **based on** the upstream `kamino/ordo` program by Kamino Finance.

- Upstream reference: [`kamino-finance/ordo`](https://github.com/Kamino-Finance/ordo) (and related Kamino Finance repositories)

## Program ID

- **Program ID**: `82qR3ftARSeXPBg9wnVrsLsAepfvimBUCPdNuiV9unhV` (see `programs/ordo/src/lib.rs`)
- **Deployments**:
  - **Mainnet-beta**: deployed at the same Program ID
  - **Devnet**: deployed at the same Program ID
  - **Localnet / Surfpool localnet**: deploy the program to your local cluster (e.g. `anchor deploy`) before running tests

## What it does (high level)

LIMO is an order-based swap/matching program that lets a **maker** create an order to swap `input_mint` for `output_mint`, and a **taker** fill (take) the order under configurable constraints. The program maintains shared configuration (global config), token vaults, and optional oracle pools used for price / staleness checks.

---

## Contract description

### Roles

- **Admin** — authority stored in `GlobalConfig.admin_authority` (and optionally `admin_authority_cached`). Can initialize config/vaults/oracle pools, update global config, withdraw host tips, and (via two-step flow) rotate admin.
- **PDA authority** — program-derived account that holds vault authority and receives tips (lamports). Derived from `global_config` key.
- **Maker** — creates limit orders (input token → output token) and can update/close them and claim maker tips.
- **Taker** — fills (takes) orders by providing output tokens and receiving input tokens; can leave a tip (split between maker and host).

### Core concepts

- **Global config** — single account per deployment: safety flags, fee BPS, oracle staleness, order-close delay, admin/pubkeys, tip accounting.
- **Vaults** — per `(global_config, mint)`: main **vault** (order input/output) and **fee_vault** (protocol fees). PDA-owned.
- **Oracle pools** — per `(global_config, token_mint)`: bind a Pyth **feed_id** (hex string) to a mint for price checks (e.g. SL max upward deviation).
- **Orders** — each order has maker, input_mint, output_mint, amounts, status (Active/Filled/Cancelled), optional TP/SL child orders, optional counterparty and permissionless flag.

### Order types

| Type           | Value | Description |
|----------------|-------|-------------|
| `Vanilla`      | 0     | Single order: swap `input_mint` → `output_mint` at fixed ratio. No TP/SL. |
| `LimitParent`  | 1     | Parent order that can have TP and/or SL **child** orders (LimitTP / LimitSL). When parent is filled, child orders use the output as their input. |
| `LimitTP`      | 2     | Take-profit child: used only as child of a LimitParent. |
| `LimitSL`      | 3     | Stop-loss child: used only as child of a LimitParent; oracle price check can enforce max upward deviation. |

- For **LimitParent**, at least one of `tp_output_amount` or `sl_output_amount` must be set; TP/SL must be enabled in global config and min distance (BPS) respected.
- **Vanilla** must have `tp_output_amount == 0` and `sl_output_amount == 0`.

### Safety switches (global config)

- **Emergency mode** — when set, blocks: `initialize_vault`, `create_order`, `update_order`, `close_order_and_claim_tip`, `take_order`, `withdraw_host_tip`.
- **Block new orders** — blocks `create_order`.
- **Block order taking** — blocks `take_order`.
- **Flash take order** — flag exists in config; flash take instructions are commented out in this repo.

### Taker permission / counterparty

- A **taker** may take an order only if:
  - `order.counterparty == default` **and** `global_config.allowed_taker == taker`, **or**
  - `taker == order.counterparty`.
- Maker can set **counterparty** and **permissionless** on the order via `update_order`.

---

## Instructions (reference)

### Admin-only instructions

#### `initialize_global_config`

- **Who:** Caller is the initial admin.
- **What:** Creates the global config account and the PDA authority (seeds: `"authority"`, `global_config` key). One-time setup per deployment.
- **Accounts:** `admin_authority` (signer), `pda_authority`, `global_config`.
- **Args:** none.

#### `initialize_oracle_pool`

- **Who:** Must match `global_config.admin_authority`.
- **What:** Registers a Pyth oracle feed for a token mint (for price/staleness checks, e.g. on SL).
- **Accounts:** `admin_authority`, `global_config`, `token_mint`, `oracle_pool`, `system_program`.
- **Args:** `feed_id: string` — Pyth feed id (hex).

#### `initialize_vault`

- **Who:** Any payer; global config must match PDA authority (effectively admin sets up vaults).
- **Constraints:** Emergency mode must be **off**.
- **What:** Creates main **vault** and **fee_vault** for a given mint under this global config. PDA is token authority.
- **Accounts:** `payer`, `global_config`, `pda_authority`, `mint`, `vault`, `fee_vault`, `token_program`, `system_program`.
- **Args:** none.

#### `update_global_config`

- **Who:** Must match `global_config.admin_authority`.
- **What:** Updates one global config field by `mode`. `value` is a 128-byte array; only the relevant prefix is used (see table below).
- **Accounts:** `admin_authority`, `global_config`.
- **Args:** `mode: u16`, `value: [u8; 128]`.

**Modes and value encoding:**

| Mode (name)                      | Value type | Encoding      | Description |
|----------------------------------|------------|---------------|-------------|
| 0  UpdateEmergencyMode           | bool       | 1 byte: 0 or 1 | Emergency switch. |
| 1  UpdateFlashTakeOrderBlocked   | bool       | 1 byte        | Block flash take (unused in this build). |
| 2  UpdateBlockNewOrders          | bool       | 1 byte        | Block new orders. |
| 3  UpdateBlockOrderTaking        | bool       | 1 byte        | Block taking orders. |
| 4  UpdateHostFeeBps              | u16        | 2 bytes LE    | Host share of tip (0–10000 BPS). |
| 5  UpdateAdminAuthorityCached    | pubkey     | 32 bytes      | New admin to apply via `update_global_config_admin`. |
| 6  UpdateOrderCloseDelaySeconds  | u64        | 8 bytes LE    | Min seconds after last update before closing order. |
| 7  UpdateTxnFeeCost              | u64        | 8 bytes LE    | Lamport cost assumed per txn. |
| 8  UpdateAtaCreationCost         | u64        | 8 bytes LE    | Lamport cost for ATA creation. |
| 9  UpdateTpSlEnabled             | bool       | 1 byte        | Allow TP/SL child orders. |
| 10 UpdateCreateOrderFeeBps       | u16        | 2 bytes LE    | Fee on create order (0–10000 BPS). |
| 11 UpdateParentFillFeeKeeperBps  | u16        | 2 bytes LE    | Keeper fee BPS on parent fill. |
| 12 UpdateParentFillFeeProtocolBps| u16        | 2 bytes LE    | Protocol fee BPS on parent fill. |
| 13 UpdateTpSlChildFeeKeeperBps   | u16        | 2 bytes LE    | Keeper fee BPS on TP/SL child fill. |
| 14 UpdateTpSlChildFeeProtocolBps | u16        | 2 bytes LE    | Protocol fee BPS on TP/SL child fill. |
| 15 UpdateOracleMaxStalenessSeconds | u64     | 8 bytes LE    | Max oracle age for price checks. |
| 16 UpdateSlMaxUpwardDeviationBps | u16        | 2 bytes LE    | SL max upward price deviation (BPS). |
| 17 UpdateTpSlMinDistanceBps       | u16        | 2 bytes LE    | Min distance between TP/SL and parent (BPS). |
| 18 UpdateAllowedTaker            | pubkey     | 32 bytes      | Global allowed taker when order has no counterparty. |

#### `update_global_config_admin`

- **Who:** Must match `global_config.admin_authority_cached`.
- **What:** Sets `admin_authority = admin_authority_cached`. Used to complete an admin rotation: first call `update_global_config` with mode `UpdateAdminAuthorityCached` and the new pubkey, then the **new** admin calls this instruction.
- **Accounts:** `admin_authority_cached` (signer), `global_config`.
- **Args:** none.

#### `withdraw_host_tip`

- **Who:** Must match `global_config.admin_authority`. Emergency mode must be **off**.
- **What:** Withdraws all accumulated host tip (lamports) from the PDA authority to the admin. Updates global config tip accounting and PDA balance tracking.
- **Accounts:** `admin_authority`, `global_config`, `pda_authority`, `system_program`.
- **Args:** none.

---

### User instructions (maker)

#### `create_order`

- **Who:** Maker (signer). Blocked if “block new orders” or emergency mode is on.
- **What:** Opens a new order: lock `input_amount` of `input_mint` in the input vault; order pays `output_amount` of `output_mint` when filled. Optionally creates TP/SL child orders (if type is LimitParent and TP/SL enabled).
- **Accounts:** Maker, global_config, pda_authority, order (and optionally tp_order, sl_order), input/output mints, maker ATA, input vault, input fee vault, optional output vault, token programs, system, event authority, program.
- **Args:** `input_amount: u64`, `output_amount: u64`, `order_type: u8`, `tp_output_amount: u64`, `sl_output_amount: u64`. For Vanilla, TP/SL amounts must be 0. For LimitParent, at least one of TP/SL must be &gt; 0 when applicable; TP/SL min distance (BPS) is enforced. A create-order fee (BPS) is applied.

#### `update_order`

- **Who:** Maker (signer). Blocked if emergency mode is on.
- **What:** Updates the order’s permissionless flag or counterparty.
- **Accounts:** `maker`, `global_config`, `order`.
- **Args:** `mode: UpdateOrderMode`, `value: bytes`.  
  - **UpdatePermissionless (0):** `value` = 1 byte, 0 or 1.  
  - **UpdateCounterparty (1):** `value` = 32 bytes (pubkey of allowed taker).

#### `close_order_and_claim_tip`

- **Who:** Maker (signer). Blocked if emergency mode is on.
- **What:** Cancels the order (and TP/SL children if present): returns remaining input to maker, closes child orders, claims maker tip to maker, updates global tip accounting. Subject to `order_close_delay_seconds`: current time must be ≥ `order.last_updated_timestamp + order_close_delay_seconds`.
- **Accounts:** Maker, order, optional tp_child_order / sl_child_order, global_config, pda_authority, input/output mints, maker input ATA, optional maker output ATA, input/output vaults, token programs, system, event authority, program.
- **Args:** none.

---

### User instructions (taker)

#### `take_order`

- **Who:** Taker (signer). Must be allowed by counterparty / allowed_taker rules. Blocked if “block order taking” or emergency mode is on.
- **What:** Fills (partially or fully) the order: taker sends output tokens to maker/vaults, receives input tokens from vault. Optional tip: part goes to maker, part to host (by host_fee_bps). For SL child orders, oracle price is checked (max staleness, max upward deviation). Minimum output is enforced for maker.
- **Accounts:** Taker, maker, global_config, pda_authority, order, optional parent_order, brother_order, input/output mints, vaults, optional oracle pools and price updates, taker input/output ATAs, optional intermediary output and maker output ATA, sysvar instructions, token programs, rent, system, event authority, program.
- **Args:** `input_amount: u64`, `min_output_amount: u64`, `tip_amount_permissionless_taking: u64`.

---

### Helper instructions (integration / testing)

- **`log_user_swap_balances_start`** / **`log_user_swap_balances_end`** — Record user balances (lamports, input TA, output TA) before/after an external swap; used with simulated amounts and aggregator ids for analytics/introspection.
- **`assert_user_swap_balances_start`** / **`assert_user_swap_balances_end`** — Assert that balance deltas stay within `max_input_amount_change` and `min_output_amount_change` (e.g. to wrap an external swap with strict limits).

These are not required for normal create/take/close flows.

---

## Main on-chain instructions (summary)

See the **Instructions (reference)** section above for full details. Quick list:

- **Admin:** `initialize_global_config`, `initialize_oracle_pool`, `initialize_vault`, `update_global_config`, `update_global_config_admin`, `withdraw_host_tip`.
- **Maker:** `create_order`, `update_order`, `close_order_and_claim_tip`.
- **Taker:** `take_order`.
- **Helpers:** `log_user_swap_balances_start` / `log_user_swap_balances_end`, `assert_user_swap_balances_start` / `assert_user_swap_balances_end`.

**Safety constraints:** Emergency mode blocks most operations; “block new orders” blocks `create_order`; “block order taking” blocks `take_order`. Order close is gated by `order_close_delay_seconds`. Taker must match `order.counterparty` or (if unset) `global_config.allowed_taker`. Flash take instructions are commented out in this repo.

## Repo layout (program)

- `programs/ordo/src/lib.rs`: program entrypoints (Anchor instructions) + error codes
- `programs/ordo/src/handlers/`: instruction handlers
- `programs/ordo/src/state/`: account state definitions (e.g., `GlobalConfig`, `Order`, `OraclePoolsState`)
- `programs/ordo/src/utils/`: constraints, constants, helper logic

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

The Anchor IDL for this program is generated into `target/idl/ordo.json`.

```bash
anchor build
ls -la target/idl/
```

## Security notes

- This codebase includes `solana-security-txt` metadata (see `programs/ordo/src/lib.rs`).


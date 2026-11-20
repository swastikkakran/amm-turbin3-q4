# Solana AMM

A constant-product Automated Market Maker (AMM) built with **Solana, Rust, and Anchor**.

This program enables users to create a liquidity pool for two SPL tokens, provide liquidity in exchange for LP tokens, swap between the pool's supported tokens, and withdraw their share of the liquidity.

The AMM follows the constant-product formula:

\[
x \cdot y = k
\]

Where:
- `x` is the reserve of token X.
- `y` is the reserve of token Y.
- `k` is the constant product that the pool maintains during swaps, before accounting for fees and integer rounding.

## Features

- **Pool initialization** — Create an AMM pool with two token mints and an LP token mint.
- **Liquidity provision** — Deposit both supported tokens into the pool and receive LP tokens representing your share of the pool.
- **Token swaps** — Swap token X for token Y or token Y for token X using the constant-product pricing model.
- **Liquidity withdrawal** — Burn LP tokens to redeem the corresponding share of the pool's reserves.
- **Protocol fees** — Collect a configurable fee on swaps and transfer it directly to a designated treasury.
- **Slippage protection** — Specify minimum output amounts for swaps and minimum withdrawal amounts.
- **On-chain validation** — Validate fee limits and reject invalid operations, including zero-amount deposits, swaps, and withdrawals.

## How It Works

### 1. Pool Initialization

The initializer creates the pool configuration and establishes the token accounts required by the AMM.

The configuration stores information such as:
- Token X mint.
- Token Y mint.
- LP token mint.
- Pool authority and vault information.
- Treasury address.
- Protocol fee in basis points.

### 2. Providing Liquidity

Liquidity providers deposit token X and token Y into the pool.

In return, the program mints LP tokens representing their proportional ownership of the pool.

LP tokens can later be burned to withdraw the provider's share of the underlying assets.

### 3. Swapping Tokens

The AMM supports swaps in both directions:

- Token X → Token Y
- Token Y → Token X

The program calculates the output using the constant-product formula and applies the protocol fee to the input amount.

The fee is separated from the amount entering the liquidity pool, so the curve calculation uses the net input amount.

### 4. Protocol Fees

The protocol fee is configured in basis points (BPS).

| Fee parameter | Value |
|---|---:|
| Denominator | 10,000 BPS |
| Maximum fee | 1,000 BPS |
| Maximum percentage | 10% |

For example, a fee of 100 BPS represents a 1% protocol fee.

The fee is transferred to the configured treasury rather than being added to the pool's reserves.

## Program Instructions

| Instruction | Description |
|---|---|
| `initialize` | Creates and configures the AMM pool. |
| `deposit` | Deposits both tokens and mints LP tokens. |
| `swap` | Swaps between token X and token Y. |
| `withdraw` | Burns LP tokens and returns the corresponding underlying assets. |

## Project Structure

The project follows the Anchor workspace structure:

```text
amm-trying/
├── programs/
│   └── amm-trying/
│       ├── src/
│       │   ├── lib.rs
│       │   ├── instructions/
│       │   └── state/
│       ├── tests/
│       │   └── test_initialize.rs
│       └── Cargo.toml
├── tests/
├── Anchor.toml
├── Cargo.toml
└── README.md
```

*The structure above illustrates the main project organization; adjust individual entries if your local source tree differs.*

## Tech Stack

- **Rust** — Smart contract implementation.
- **Solana** — Blockchain runtime.
- **Anchor** — Solana program framework.
- **SPL Token** — Token transfers, minting, and burning.
- **LiteSVM** — Lightweight Solana virtual machine for program testing.

## Getting Started

### Prerequisites

Install the following tools:

- Rust
- Solana CLI
- Anchor CLI
- Node.js and Yarn, if required by your Anchor workspace

### Build

Clone the repository:

```bash
git clone <YOUR_REPOSITORY_URL>
cd amm-trying
```

Build the program:

```bash
anchor build
```

### Run Tests

Run the AMM integration tests:

```bash
cargo test -p amm-trying --test test_initialize -- --nocapture
```

Run all workspace tests:

```bash
cargo test --workspace
```

## Testing

The program includes LiteSVM integration tests covering the main AMM lifecycle and important failure cases.

The test suite covers:

- Pool initialization and configuration.
- Full liquidity lifecycle.
- Deposits and LP token minting.
- Withdrawals and LP token burning.
- Swaps in both directions.
- Treasury fee collection.
- Correct reserve and token balance accounting.
- Invalid fee configuration.
- Zero-amount deposits, swaps, and withdrawals.
- Slippage failures and transaction rollback.

These tests exercise the program against a Solana runtime rather than relying solely on isolated unit tests.

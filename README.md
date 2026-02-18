# solana-fib-cpi

Minimal Solana program that computes a Fibonacci sequence using recursive self-CPI.

## How it works

A single instruction handles two modes based on PDA state:

**Init** (PDA doesn't exist) — pass `n` as a u64 LE in instruction data:
1. Creates a PDA derived from `["fib", payer_pubkey]`
2. Stores initial state: `a=0, b=1, n=N, bump`
3. If `n > 0`, invokes itself via CPI to start stepping

**Step** (PDA exists) — empty instruction data:
1. Reads `(a, b, n)` from the PDA
2. Writes `(b, a+b, n-1)` — standard Fibonacci advance
3. If steps remain, invokes itself again recursively

### Accounts

| # | Account        | Writable | Signer |
|---|----------------|----------|--------|
| 0 | PDA            | yes      | no     |
| 1 | Payer          | yes      | yes    |
| 2 | System Program | no       | no     |

### PDA data layout (25 bytes)

| Offset | Size | Field |
|--------|------|-------|
| 0      | 8    | a (u64, previous Fibonacci number) |
| 8      | 8    | b (u64, current Fibonacci number)  |
| 16     | 8    | n (u64, remaining steps)           |
| 24     | 1    | bump seed                          |

### CPI depth limit

Solana's max invoke stack height is 5. Since init runs at height 1, up to 4 recursive steps can execute in a single transaction (`n <= 4`). For larger values, call the program multiple times from the client.

## Prerequisites

- [Rust](https://rustup.rs/)
- [Solana CLI tools](https://docs.solana.com/cli/install-solana-cli-tools) (provides `cargo build-sbf` and `cargo test-sbf`)

## Build

```
cargo build-sbf
```

The compiled program is output to `target/deploy/solana-fib-cpi.so`.

## Test

```
cargo test-sbf
```

Run a specific test:

```
cargo test-sbf -- fibonacci_n3
```

## Dependencies

Uses only the modular Solana v2 crates — no Anchor, no monolithic `solana-program`:

- `solana-account-info`
- `solana-cpi`
- `solana-instruction`
- `solana-msg`
- `solana-program-entrypoint`
- `solana-pubkey`

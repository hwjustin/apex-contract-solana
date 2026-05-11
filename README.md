# APEX-Contract

## Quick Start

Setup Foundry
```
curl -L https://foundry.paradigm.xyz | bash
foundryup
cd contracts
forge install foundry-rs/forge-std
```

Setup Environmental Variables
```
BASE_RPC_URL=
PRIVATE_KEY=
BASESCAN_API_KEY=
```

```
set -a
source .env
set +a
```

Deploy the contracts

```
cd contracts

forge build

forge script script/Deploy.s.sol \
  --rpc-url base \
  --broadcast \
  --verify

forge script script/DeployDemo.s.sol \
  --rpc-url base \
  --broadcast \
  --verify

forge script script/CreateProduct.s.sol \
  --rpc-url base \
  --broadcast
```

## Solana Port

A first-pass Anchor workspace for a Solana migration now lives in `solana/`. It mirrors the five registry modules described in the migration deck using PDA-based state and SPL-token escrow. See `solana/README.md` for details.

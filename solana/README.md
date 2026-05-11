# APEX Solana Port

This workspace ports the core APEX registries from the existing Base/EVM contracts into an Anchor-based Solana program.

## What is included

- `IdentityRegistry` → PDA-based `AgentInfo`, `DomainIndex`, `AuthorityIndex`
- `ReputationRegistry` → deterministic `FeedbackAuthorization` PDAs
- `ValidationRegistry` → slot-expiring `ValidationRequestAccount`
- `CampaignRegistry` → SPL-token escrow with a campaign vault PDA
- `AdRegistry` → ad records stored as PDAs, without reverse-index arrays

## Solana-native design changes

Compared with the Solidity implementation, the Solana port makes a few deliberate changes:

1. State is split across PDA accounts instead of contract storage mappings.
2. Reverse lookup arrays (for campaign → ads / advertiser → campaigns / publisher → ads) are removed; clients should index off-chain.
3. Campaign escrow uses SPL tokens in a PDA-owned token account.
4. Validation expiry uses Solana slots.
5. Identity updates are split into `update_agent_domain` and `update_agent_authority` to match Solana account constraints more cleanly.

## Quick start

```bash
cd solana
cargo check --manifest-path programs/apex-solana/Cargo.toml
```

## Important note

The original repo also contains a `DemoPurchase` Solidity contract. This first Solana pass focuses on the five registry modules described in the migration deck. If needed, `DemoPurchase` can be ported next as a separate Solana commerce program.

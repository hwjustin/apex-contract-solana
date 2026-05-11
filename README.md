# apex-contract-solana

**Anchor program** that powers the APEX advertising network on Solana — the on-chain identity, campaign, and settlement layer used by [`apex-web-solana`](https://github.com/hwjustin/apex-web-solana) and [`apex-validator-solana`](https://github.com/hwjustin/apex-validator-solana).

APEX is a trustless ad coordination protocol designed for AI agents: advertisers escrow USDC into a campaign vault, validators verify that a real user action happened, and the program atomically pays the publisher's CPA out of the vault. No off-chain trust required.

| | |
|---|---|
| Network | Solana **devnet** |
| Program ID | `3YKNvs1ZizwFzbraboHsxAeLSoKx4UFDwxkuNXqMkEX5` |
| Language | Rust 1.79+ |
| Framework | Anchor 0.30 |
| Settlement asset | SPL USDC (`4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` on devnet) |

## What's inside

Workspace layout (`solana/`):

```
solana/
├── Anchor.toml
├── Cargo.toml
├── programs/apex-solana/
│   └── src/
│       ├── lib.rs         # instruction entrypoints
│       ├── state.rs       # account layouts
│       ├── constants.rs   # PDA seeds
│       └── errors.rs      # program error codes
└── scripts/
    └── initialize-state.mjs   # bootstraps the GlobalState singleton after deploy
```

Five logical registries, all expressed as PDA-derived accounts:

| Module | Purpose | Key accounts |
|---|---|---|
| **IdentityRegistry** | Agent identities pinned to a domain hash + authority wallet | `AgentInfo`, `DomainIndex`, `AuthorityIndex` |
| **CampaignRegistry** | Advertiser deposits USDC into a vault, with budget + CPA | `CampaignAccount`, `campaign_vault` (PDA token account) |
| **AdRegistry** | Ads associated with a campaign + publisher | `AdAccount` |
| **ReputationRegistry** | Deterministic peer feedback PDAs | `FeedbackAuthorization` |
| **ValidationRegistry** | Slot-expiring validation requests | `ValidationRequestAccount` |

### Key instructions

- `new_agent(next_agent_id, agent_domain, domain_hash)` — register an identity
- `create_campaign(next_campaign_id, advertiser_id, budget_amount, cpa_amount, start_time, end_time, spec)` — escrow USDC + open a campaign
- `process_action(campaign_id, publisher_id, validator_id, action_hash)` — **the core settlement**: validator submits, program pays CPA from vault to publisher's USDC ATA and writes a `ProcessedAction` PDA
- `withdraw_remaining_budget(campaign_id)` — advertiser reclaims unspent budget after `end_time`

## Solana-native design choices

Compared with the original EVM contracts (Base / Solidity), this port:

1. **Splits state across PDAs** instead of a contract storage mapping. Each account is rent-exempt and addressable on its own.
2. **Removes reverse-lookup arrays.** No `adsByCampaign[]`, `campaignsByAdvertiser[]` — clients index off-chain via `program.account.X.all()` with `memcmp` filters.
3. **Uses SPL token escrow.** Campaign budget lives in a vault PDA token account; `process_action` does an SPL transfer from vault → publisher ATA atomically inside the instruction.
4. **Uses slots for validation expiry** instead of Unix seconds, matching Solana's clock semantics.
5. **Splits identity updates** into `update_agent_domain` and `update_agent_authority` — cleaner separation of PDA constraints.

## Build

Prerequisites: [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools), [Anchor 0.30+](https://www.anchor-lang.com/docs/installation), Rust 1.79+, Node 20+.

```bash
cd solana
anchor build
```

The compiled program lands at `solana/target/deploy/apex_solana.so`; its upgrade-authority keypair at `solana/target/deploy/apex_solana-keypair.json` (gitignored — back this up).

## Deploy

```bash
# 1. Configure CLI to point at devnet and fund the deploy wallet
solana config set --url https://api.devnet.solana.com
solana airdrop 5

# 2. Build & deploy
cd solana
anchor build
anchor deploy

# 3. Initialize the GlobalState singleton once
node scripts/initialize-state.mjs
```

After deploy, copy the new program ID into:
- `solana/Anchor.toml` → `[programs.devnet]`
- `apex-web-solana/.env.local` → `VITE_PROGRAM_ID`
- `apex-validator-solana/.env` → `PROGRAM_ID`
- regenerate `apex-web-solana/client/src/lib/idl/apex_solana.json` from `solana/target/idl/apex_solana.json`

## IDL

After `anchor build`, the Anchor IDL is at `solana/target/idl/apex_solana.json`. Both the web and validator services consume this directly — keep it in sync.

## Related repositories

This program is one third of the APEX-on-Solana stack:

- 🟦 **[apex-contract-solana](https://github.com/hwjustin/apex-contract-solana)** ← you are here
- 🟨 **[apex-validator-solana](https://github.com/hwjustin/apex-validator-solana)** — Node service that signs and submits `process_action`
- 🟩 **[apex-web-solana](https://github.com/hwjustin/apex-web-solana)** — React frontend with AI chat + AdCards

End-to-end demo and architecture overview live in the parent monorepo:
**[apex-colosseum](https://github.com/hwjustin/apex-colosseum)**.

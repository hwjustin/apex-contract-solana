pub const STATE_SEED: &[u8] = b"state";
pub const AGENT_SEED: &[u8] = b"agent";
pub const DOMAIN_SEED: &[u8] = b"domain";
pub const AUTHORITY_SEED: &[u8] = b"authority";
pub const FEEDBACK_SEED: &[u8] = b"feedback";
pub const VALIDATION_SEED: &[u8] = b"validation";
pub const CAMPAIGN_SEED: &[u8] = b"campaign";
pub const CAMPAIGN_VAULT_SEED: &[u8] = b"campaign-vault";
pub const PROCESSED_ACTION_SEED: &[u8] = b"processed-action";
pub const AD_SEED: &[u8] = b"ad";

pub const MAX_AGENT_DOMAIN_LEN: usize = 128;
pub const MAX_CAMPAIGN_SPEC_LEN: usize = 2048;
pub const MAX_AD_METADATA_LEN: usize = 2048;
pub const VALIDATION_EXPIRATION_SLOTS: u64 = 1_000;

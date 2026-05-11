use anchor_lang::prelude::*;
use crate::constants::*;

#[account]
#[derive(InitSpace)]
pub struct GlobalState {
    pub authority: Pubkey,
    pub registration_fee_lamports: u64,
    pub agent_count: u64,
    pub campaign_count: u64,
    pub ad_count: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AgentInfo {
    pub agent_id: u64,
    #[max_len(MAX_AGENT_DOMAIN_LEN)]
    pub agent_domain: String,
    pub authority: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct DomainIndex {
    pub agent_id: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AuthorityIndex {
    pub agent_id: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct FeedbackAuthorization {
    pub client_agent_id: u64,
    pub server_agent_id: u64,
    pub feedback_auth_id: [u8; 32],
    pub authorized_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ValidationRequestAccount {
    pub validator_agent_id: u64,
    pub server_agent_id: u64,
    pub data_hash: [u8; 32],
    pub requested_slot: u64,
    pub expiry_slot: u64,
    pub responded: bool,
    pub response: u8,
    pub responder: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct CampaignAccount {
    pub campaign_id: u64,
    pub advertiser_id: u64,
    pub authority: Pubkey,
    pub budget_amount: u64,
    pub spent_amount: u64,
    pub cpa_amount: u64,
    pub budget_mint: Pubkey,
    pub start_time: i64,
    pub expiry_time: i64,
    #[max_len(MAX_CAMPAIGN_SPEC_LEN)]
    pub spec: Vec<u8>,
    pub vault_bump: u8,
    pub bump: u8,
}

impl CampaignAccount {
    pub fn remaining_budget(&self) -> Result<u64> {
        self.budget_amount
            .checked_sub(self.spent_amount)
            .ok_or_else(|| error!(crate::errors::ApexError::Overflow))
    }

    pub fn is_active(&self, now: i64) -> Result<bool> {
        Ok(now >= self.start_time && now < self.expiry_time && self.remaining_budget()? >= self.cpa_amount)
    }
}

#[account]
#[derive(InitSpace)]
pub struct ProcessedAction {
    pub campaign_id: u64,
    pub publisher_id: u64,
    pub validator_id: u64,
    pub action_hash: [u8; 32],
    pub payment_amount: u64,
    pub processed_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AdAccount {
    pub ad_id: u64,
    pub campaign_id: u64,
    pub advertiser_id: u64,
    pub publisher_id: u64,
    pub start_time: i64,
    #[max_len(MAX_AD_METADATA_LEN)]
    pub metadata: Vec<u8>,
    pub bump: u8,
}

use anchor_lang::prelude::*;

#[error_code]
pub enum ApexError {
    #[msg("Invalid agent domain")]
    InvalidDomain,
    #[msg("Invalid domain hash")]
    InvalidDomainHash,
    #[msg("Agent id does not match the next available id")]
    InvalidNextAgentId,
    #[msg("Campaign id does not match the next available id")]
    InvalidNextCampaignId,
    #[msg("Ad id does not match the next available id")]
    InvalidNextAdId,
    #[msg("Unauthorized caller")]
    Unauthorized,
    #[msg("Agent authority mismatch")]
    InvalidAgentAuthority,
    #[msg("Campaign already expired")]
    CampaignAlreadyExpired,
    #[msg("Invalid campaign time range")]
    InvalidTimeRange,
    #[msg("Invalid budget amount")]
    InvalidBudgetAmount,
    #[msg("Invalid CPA amount")]
    InvalidCpaAmount,
    #[msg("Campaign is not active")]
    CampaignNotActive,
    #[msg("Campaign is still active")]
    CampaignStillActive,
    #[msg("Insufficient remaining campaign budget")]
    InsufficientBudget,
    #[msg("Action has already been processed")]
    ActionAlreadyProcessed,
    #[msg("Ad start time cannot be in the future")]
    InvalidStartTime,
    #[msg("Metadata exceeds maximum length")]
    MetadataTooLarge,
    #[msg("Campaign spec exceeds maximum length")]
    SpecTooLarge,
    #[msg("Validation request not found")]
    ValidationRequestNotFound,
    #[msg("Validation request is expired")]
    ValidationRequestExpired,
    #[msg("Validation already responded")]
    ValidationAlreadyResponded,
    #[msg("Invalid validation response")]
    InvalidValidationResponse,
    #[msg("Registration fee mismatch")]
    InvalidRegistrationFee,
    #[msg("Arithmetic overflow")]
    Overflow,
}

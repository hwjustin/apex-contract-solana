use anchor_lang::prelude::*;
use anchor_lang::solana_program::{hash::{hash, hashv}, program::invoke, system_instruction};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

pub mod constants;
pub mod errors;
pub mod state;

use constants::*;
use errors::ApexError;
use state::*;

declare_id!("3YKNvs1ZizwFzbraboHsxAeLSoKx4UFDwxkuNXqMkEX5");

#[program]
pub mod apex_solana {
    use super::*;

    pub fn initialize_state(ctx: Context<InitializeState>, registration_fee_lamports: u64) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.authority = ctx.accounts.authority.key();
        state.registration_fee_lamports = registration_fee_lamports;
        state.agent_count = 0;
        state.campaign_count = 0;
        state.ad_count = 0;
        state.bump = ctx.bumps.state;
        Ok(())
    }

    pub fn new_agent(
        ctx: Context<NewAgent>,
        next_agent_id: u64,
        agent_domain: String,
        domain_hash: [u8; 32],
    ) -> Result<()> {
        require!(!agent_domain.is_empty(), ApexError::InvalidDomain);
        require!(agent_domain.len() <= MAX_AGENT_DOMAIN_LEN, ApexError::InvalidDomain);
        let expected_hash = hash(agent_domain.as_bytes()).to_bytes();
        require!(expected_hash == domain_hash, ApexError::InvalidDomainHash);

        let expected_id = ctx.accounts.state.agent_count.checked_add(1).ok_or_else(|| error!(ApexError::Overflow))?;
        require!(next_agent_id == expected_id, ApexError::InvalidNextAgentId);
        let registration_fee_lamports = ctx.accounts.state.registration_fee_lamports;
        let state_key = ctx.accounts.state.key();

        if registration_fee_lamports > 0 {
            invoke(
                &system_instruction::transfer(
                    &ctx.accounts.authority.key(),
                    &state_key,
                    registration_fee_lamports,
                ),
                &[
                    ctx.accounts.authority.to_account_info(),
                    ctx.accounts.state.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
        }

        let agent = &mut ctx.accounts.agent;
        agent.agent_id = next_agent_id;
        agent.agent_domain = agent_domain;
        agent.authority = ctx.accounts.authority.key();
        agent.bump = ctx.bumps.agent;

        let domain_index = &mut ctx.accounts.domain_index;
        domain_index.agent_id = next_agent_id;
        domain_index.bump = ctx.bumps.domain_index;

        let authority_index = &mut ctx.accounts.authority_index;
        authority_index.agent_id = next_agent_id;
        authority_index.bump = ctx.bumps.authority_index;

        ctx.accounts.state.agent_count = expected_id;
        Ok(())
    }

    pub fn update_agent_domain(
        ctx: Context<UpdateAgentDomain>,
        _agent_id: u64,
        old_domain_hash: [u8; 32],
        new_domain: String,
        new_domain_hash: [u8; 32],
    ) -> Result<()> {
        require!(!new_domain.is_empty(), ApexError::InvalidDomain);
        require!(new_domain.len() <= MAX_AGENT_DOMAIN_LEN, ApexError::InvalidDomain);
        require!(hash(ctx.accounts.agent.agent_domain.as_bytes()).to_bytes() == old_domain_hash, ApexError::InvalidDomainHash);
        require!(hash(new_domain.as_bytes()).to_bytes() == new_domain_hash, ApexError::InvalidDomainHash);
        require_keys_eq!(ctx.accounts.agent.authority, ctx.accounts.authority.key(), ApexError::Unauthorized);

        ctx.accounts.agent.agent_domain = new_domain;
        ctx.accounts.new_domain_index.agent_id = ctx.accounts.agent.agent_id;
        ctx.accounts.new_domain_index.bump = ctx.bumps.new_domain_index;
        Ok(())
    }

    pub fn update_agent_authority(
        ctx: Context<UpdateAgentAuthority>,
        _agent_id: u64,
        new_authority: Pubkey,
    ) -> Result<()> {
        require_keys_eq!(ctx.accounts.agent.authority, ctx.accounts.current_authority.key(), ApexError::Unauthorized);
        ctx.accounts.agent.authority = new_authority;
        ctx.accounts.new_authority_index.agent_id = ctx.accounts.agent.agent_id;
        ctx.accounts.new_authority_index.bump = ctx.bumps.new_authority_index;
        Ok(())
    }

    pub fn accept_feedback(
        ctx: Context<AcceptFeedback>,
        client_agent_id: u64,
        server_agent_id: u64,
    ) -> Result<()> {
        require_keys_eq!(ctx.accounts.server_agent.authority, ctx.accounts.server_authority.key(), ApexError::Unauthorized);

        let clock = Clock::get()?;

        // Mix slot, unix_timestamp, and server authority into the auth id so it
        // cannot be predicted purely from the deterministic PDA seeds — mirrors
        // the EVM keccak256(client, server, blockTimestamp, blockDifficulty, txOrigin).
        let feedback_auth_id = hashv(&[
            FEEDBACK_SEED,
            &client_agent_id.to_le_bytes(),
            &server_agent_id.to_le_bytes(),
            &clock.slot.to_le_bytes(),
            &clock.unix_timestamp.to_le_bytes(),
            ctx.accounts.server_authority.key().as_ref(),
            ctx.accounts.feedback_authorization.key().as_ref(),
        ]);

        let feedback = &mut ctx.accounts.feedback_authorization;
        feedback.client_agent_id = client_agent_id;
        feedback.server_agent_id = server_agent_id;
        feedback.feedback_auth_id = feedback_auth_id.to_bytes();
        feedback.authorized_at = clock.unix_timestamp;
        feedback.bump = ctx.bumps.feedback_authorization;
        Ok(())
    }

    pub fn validation_request(
        ctx: Context<ValidationRequest>,
        validator_agent_id: u64,
        server_agent_id: u64,
        data_hash: [u8; 32],
    ) -> Result<()> {
        let clock = Clock::get()?;
        let request = &mut ctx.accounts.validation_request;

        if request.data_hash != [0u8; 32] && clock.slot <= request.expiry_slot {
            return Ok(());
        }

        request.validator_agent_id = validator_agent_id;
        request.server_agent_id = server_agent_id;
        request.data_hash = data_hash;
        request.requested_slot = clock.slot;
        request.expiry_slot = clock.slot.checked_add(VALIDATION_EXPIRATION_SLOTS).ok_or_else(|| error!(ApexError::Overflow))?;
        request.responded = false;
        request.response = 0;
        request.responder = Pubkey::default();
        request.bump = ctx.bumps.validation_request;
        Ok(())
    }

    pub fn validation_response(
        ctx: Context<ValidationResponse>,
        _data_hash: [u8; 32],
        response: u8,
    ) -> Result<()> {
        require!(response <= 100, ApexError::InvalidValidationResponse);
        let clock = Clock::get()?;
        let request = &mut ctx.accounts.validation_request;
        require!(request.data_hash != [0u8; 32], ApexError::ValidationRequestNotFound);
        require!(clock.slot <= request.expiry_slot, ApexError::ValidationRequestExpired);
        require!(!request.responded, ApexError::ValidationAlreadyResponded);
        require_keys_eq!(ctx.accounts.validator_agent.authority, ctx.accounts.validator_authority.key(), ApexError::Unauthorized);

        request.responded = true;
        request.response = response;
        request.responder = ctx.accounts.validator_authority.key();
        Ok(())
    }

    pub fn create_campaign(
        ctx: Context<CreateCampaign>,
        next_campaign_id: u64,
        advertiser_id: u64,
        budget_amount: u64,
        cpa_amount: u64,
        start_time: i64,
        expiry_time: i64,
        spec: Vec<u8>,
    ) -> Result<()> {
        require!(budget_amount > 0, ApexError::InvalidBudgetAmount);
        require!(cpa_amount > 0 && cpa_amount <= budget_amount, ApexError::InvalidCpaAmount);
        require!(start_time < expiry_time, ApexError::InvalidTimeRange);
        require!(expiry_time > Clock::get()?.unix_timestamp, ApexError::CampaignAlreadyExpired);
        require!(spec.len() <= MAX_CAMPAIGN_SPEC_LEN, ApexError::SpecTooLarge);
        require_keys_eq!(ctx.accounts.advertiser_agent.authority, ctx.accounts.advertiser.key(), ApexError::Unauthorized);

        let expected_id = ctx.accounts.state.campaign_count.checked_add(1).ok_or_else(|| error!(ApexError::Overflow))?;
        require!(next_campaign_id == expected_id, ApexError::InvalidNextCampaignId);

        token::transfer(ctx.accounts.transfer_to_vault_ctx(), budget_amount)?;

        let campaign = &mut ctx.accounts.campaign;
        campaign.campaign_id = next_campaign_id;
        campaign.advertiser_id = advertiser_id;
        campaign.authority = ctx.accounts.advertiser.key();
        campaign.budget_amount = budget_amount;
        campaign.spent_amount = 0;
        campaign.cpa_amount = cpa_amount;
        campaign.budget_mint = ctx.accounts.budget_mint.key();
        campaign.start_time = start_time;
        campaign.expiry_time = expiry_time;
        campaign.spec = spec;
        campaign.vault_bump = ctx.bumps.campaign_vault;
        campaign.bump = ctx.bumps.campaign;

        ctx.accounts.state.campaign_count = expected_id;
        Ok(())
    }

    pub fn update_campaign(
        ctx: Context<UpdateCampaign>,
        _campaign_id: u64,
        cpa_amount: u64,
        start_time: i64,
        expiry_time: i64,
        spec: Vec<u8>,
    ) -> Result<()> {
        require!(spec.len() <= MAX_CAMPAIGN_SPEC_LEN, ApexError::SpecTooLarge);
        require!(start_time < expiry_time, ApexError::InvalidTimeRange);
        require!(expiry_time > Clock::get()?.unix_timestamp, ApexError::CampaignAlreadyExpired);
        require_keys_eq!(ctx.accounts.campaign.authority, ctx.accounts.advertiser.key(), ApexError::Unauthorized);

        let campaign = &mut ctx.accounts.campaign;
        let remaining = campaign.remaining_budget()?;
        require!(cpa_amount > 0 && cpa_amount <= remaining, ApexError::InvalidCpaAmount);
        campaign.cpa_amount = cpa_amount;
        campaign.start_time = start_time;
        campaign.expiry_time = expiry_time;
        campaign.spec = spec;
        Ok(())
    }

    pub fn process_action(
        ctx: Context<ProcessAction>,
        campaign_id: u64,
        publisher_id: u64,
        validator_id: u64,
        action_hash: [u8; 32],
    ) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(ctx.accounts.campaign.is_active(now)?, ApexError::CampaignNotActive);
        require!(ctx.accounts.campaign.remaining_budget()? >= ctx.accounts.campaign.cpa_amount, ApexError::InsufficientBudget);

        let campaign = &mut ctx.accounts.campaign;
        let cpa = campaign.cpa_amount;
        campaign.spent_amount = campaign.spent_amount.checked_add(cpa).ok_or_else(|| error!(ApexError::Overflow))?;

        let processed = &mut ctx.accounts.processed_action;
        processed.campaign_id = campaign_id;
        processed.publisher_id = publisher_id;
        processed.validator_id = validator_id;
        processed.action_hash = action_hash;
        processed.payment_amount = cpa;
        processed.processed_at = now;
        processed.bump = ctx.bumps.processed_action;

        let signer_seeds: &[&[&[u8]]] = &[&[
            CAMPAIGN_SEED,
            &campaign.campaign_id.to_le_bytes(),
            &[campaign.bump],
        ]];
        token::transfer(ctx.accounts.transfer_to_publisher_ctx().with_signer(signer_seeds), cpa)?;
        Ok(())
    }

    pub fn withdraw_remaining_budget(ctx: Context<WithdrawRemainingBudget>, _campaign_id: u64) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let campaign = &mut ctx.accounts.campaign;
        require_keys_eq!(campaign.authority, ctx.accounts.advertiser.key(), ApexError::Unauthorized);
        require!(now >= campaign.expiry_time, ApexError::CampaignStillActive);

        let remaining = campaign.remaining_budget()?;
        require!(remaining > 0, ApexError::InsufficientBudget);
        campaign.spent_amount = campaign.budget_amount;

        let signer_seeds: &[&[&[u8]]] = &[&[
            CAMPAIGN_SEED,
            &campaign.campaign_id.to_le_bytes(),
            &[campaign.bump],
        ]];
        token::transfer(ctx.accounts.withdraw_ctx().with_signer(signer_seeds), remaining)?;
        Ok(())
    }

    pub fn create_ad(
        ctx: Context<CreateAd>,
        next_ad_id: u64,
        campaign_id: u64,
        publisher_id: u64,
        start_time: i64,
        metadata: Vec<u8>,
    ) -> Result<()> {
        require!(metadata.len() <= MAX_AD_METADATA_LEN, ApexError::MetadataTooLarge);
        let now = Clock::get()?.unix_timestamp;
        require!(start_time <= now, ApexError::InvalidStartTime);
        require!(ctx.accounts.campaign.is_active(now)?, ApexError::CampaignNotActive);

        let caller = ctx.accounts.caller.key();
        let advertiser_authority = ctx.accounts.advertiser_agent.authority;
        let publisher_authority = ctx.accounts.publisher_agent.authority;
        require!(caller == advertiser_authority || caller == publisher_authority, ApexError::Unauthorized);

        let state = &mut ctx.accounts.state;
        let expected_id = state.ad_count.checked_add(1).ok_or_else(|| error!(ApexError::Overflow))?;
        require!(next_ad_id == expected_id, ApexError::InvalidNextAdId);

        let ad = &mut ctx.accounts.ad;
        ad.ad_id = next_ad_id;
        ad.campaign_id = campaign_id;
        ad.advertiser_id = ctx.accounts.campaign.advertiser_id;
        ad.publisher_id = publisher_id;
        ad.start_time = start_time;
        ad.metadata = metadata;
        ad.bump = ctx.bumps.ad;

        state.ad_count = expected_id;
        Ok(())
    }

    pub fn update_ad(ctx: Context<UpdateAd>, _ad_id: u64, metadata: Vec<u8>) -> Result<()> {
        require!(metadata.len() <= MAX_AD_METADATA_LEN, ApexError::MetadataTooLarge);
        require_keys_eq!(ctx.accounts.advertiser_agent.authority, ctx.accounts.advertiser.key(), ApexError::Unauthorized);
        ctx.accounts.ad.metadata = metadata;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeState<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + GlobalState::INIT_SPACE,
        seeds = [STATE_SEED],
        bump
    )]
    pub state: Account<'info, GlobalState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(next_agent_id: u64, _agent_domain: String, domain_hash: [u8; 32])]
pub struct NewAgent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, seeds = [STATE_SEED], bump = state.bump)]
    pub state: Account<'info, GlobalState>,
    #[account(
        init,
        payer = authority,
        space = 8 + AgentInfo::INIT_SPACE,
        seeds = [AGENT_SEED, &next_agent_id.to_le_bytes()],
        bump
    )]
    pub agent: Account<'info, AgentInfo>,
    #[account(
        init,
        payer = authority,
        space = 8 + DomainIndex::INIT_SPACE,
        seeds = [DOMAIN_SEED, &domain_hash],
        bump
    )]
    pub domain_index: Account<'info, DomainIndex>,
    #[account(
        init,
        payer = authority,
        space = 8 + AuthorityIndex::INIT_SPACE,
        seeds = [AUTHORITY_SEED, authority.key().as_ref()],
        bump
    )]
    pub authority_index: Account<'info, AuthorityIndex>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(agent_id: u64, old_domain_hash: [u8; 32], _new_domain: String, new_domain_hash: [u8; 32])]
pub struct UpdateAgentDomain<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [AGENT_SEED, &agent_id.to_le_bytes()],
        bump = agent.bump,
    )]
    pub agent: Account<'info, AgentInfo>,
    #[account(
        mut,
        seeds = [DOMAIN_SEED, &old_domain_hash],
        bump = old_domain_index.bump,
        close = authority,
    )]
    pub old_domain_index: Account<'info, DomainIndex>,
    #[account(
        init,
        payer = authority,
        space = 8 + DomainIndex::INIT_SPACE,
        seeds = [DOMAIN_SEED, &new_domain_hash],
        bump,
    )]
    pub new_domain_index: Account<'info, DomainIndex>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(agent_id: u64, new_authority: Pubkey)]
pub struct UpdateAgentAuthority<'info> {
    #[account(mut)]
    pub current_authority: Signer<'info>,
    /// CHECK: new authority can be any pubkey
    pub authority_target: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [AGENT_SEED, &agent_id.to_le_bytes()],
        bump = agent.bump,
    )]
    pub agent: Account<'info, AgentInfo>,
    #[account(
        mut,
        seeds = [AUTHORITY_SEED, current_authority.key().as_ref()],
        bump = current_authority_index.bump,
        close = current_authority,
    )]
    pub current_authority_index: Account<'info, AuthorityIndex>,
    #[account(
        init,
        payer = current_authority,
        space = 8 + AuthorityIndex::INIT_SPACE,
        seeds = [AUTHORITY_SEED, new_authority.as_ref()],
        bump,
    )]
    pub new_authority_index: Account<'info, AuthorityIndex>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(client_agent_id: u64, server_agent_id: u64)]
pub struct AcceptFeedback<'info> {
    #[account(mut)]
    pub server_authority: Signer<'info>,
    #[account(seeds = [AGENT_SEED, &client_agent_id.to_le_bytes()], bump = client_agent.bump)]
    pub client_agent: Account<'info, AgentInfo>,
    #[account(seeds = [AGENT_SEED, &server_agent_id.to_le_bytes()], bump = server_agent.bump)]
    pub server_agent: Account<'info, AgentInfo>,
    #[account(
        init,
        payer = server_authority,
        space = 8 + FeedbackAuthorization::INIT_SPACE,
        seeds = [FEEDBACK_SEED, &client_agent_id.to_le_bytes(), &server_agent_id.to_le_bytes()],
        bump,
    )]
    pub feedback_authorization: Account<'info, FeedbackAuthorization>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(validator_agent_id: u64, server_agent_id: u64, data_hash: [u8; 32])]
pub struct ValidationRequest<'info> {
    #[account(mut)]
    pub requester: Signer<'info>,
    #[account(seeds = [AGENT_SEED, &validator_agent_id.to_le_bytes()], bump = validator_agent.bump)]
    pub validator_agent: Account<'info, AgentInfo>,
    #[account(seeds = [AGENT_SEED, &server_agent_id.to_le_bytes()], bump = server_agent.bump)]
    pub server_agent: Account<'info, AgentInfo>,
    #[account(
        init_if_needed,
        payer = requester,
        space = 8 + ValidationRequestAccount::INIT_SPACE,
        seeds = [VALIDATION_SEED, &data_hash],
        bump,
    )]
    pub validation_request: Account<'info, ValidationRequestAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(data_hash: [u8; 32], _response: u8)]
pub struct ValidationResponse<'info> {
    #[account(mut)]
    pub validator_authority: Signer<'info>,
    #[account(mut, seeds = [VALIDATION_SEED, &data_hash], bump = validation_request.bump)]
    pub validation_request: Account<'info, ValidationRequestAccount>,
    #[account(
        seeds = [AGENT_SEED, &validation_request.validator_agent_id.to_le_bytes()],
        bump = validator_agent.bump,
    )]
    pub validator_agent: Account<'info, AgentInfo>,
}

#[derive(Accounts)]
#[instruction(next_campaign_id: u64, advertiser_id: u64, _budget_amount: u64, _cpa_amount: u64, _start_time: i64, _expiry_time: i64, _spec: Vec<u8>)]
pub struct CreateCampaign<'info> {
    #[account(mut)]
    pub advertiser: Signer<'info>,
    #[account(mut, seeds = [STATE_SEED], bump = state.bump)]
    pub state: Box<Account<'info, GlobalState>>,
    #[account(seeds = [AGENT_SEED, &advertiser_id.to_le_bytes()], bump = advertiser_agent.bump)]
    pub advertiser_agent: Box<Account<'info, AgentInfo>>,
    pub budget_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        constraint = advertiser_token_account.owner == advertiser.key(),
        constraint = advertiser_token_account.mint == budget_mint.key(),
    )]
    pub advertiser_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = advertiser,
        space = 8 + CampaignAccount::INIT_SPACE,
        seeds = [CAMPAIGN_SEED, &next_campaign_id.to_le_bytes()],
        bump,
    )]
    pub campaign: Box<Account<'info, CampaignAccount>>,
    #[account(
        init,
        payer = advertiser,
        token::mint = budget_mint,
        token::authority = campaign,
        seeds = [CAMPAIGN_VAULT_SEED, &next_campaign_id.to_le_bytes()],
        bump,
    )]
    pub campaign_vault: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> CreateCampaign<'info> {
    fn transfer_to_vault_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.advertiser_token_account.to_account_info(),
            to: self.campaign_vault.to_account_info(),
            authority: self.advertiser.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
#[instruction(campaign_id: u64, _cpa_amount: u64, _start_time: i64, _expiry_time: i64, _spec: Vec<u8>)]
pub struct UpdateCampaign<'info> {
    pub advertiser: Signer<'info>,
    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, &campaign_id.to_le_bytes()],
        bump = campaign.bump,
    )]
    pub campaign: Account<'info, CampaignAccount>,
}

#[derive(Accounts)]
#[instruction(campaign_id: u64, publisher_id: u64, validator_id: u64, action_hash: [u8; 32])]
pub struct ProcessAction<'info> {
    #[account(mut)]
    pub validator: Signer<'info>,
    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, &campaign_id.to_le_bytes()],
        bump = campaign.bump,
    )]
    pub campaign: Account<'info, CampaignAccount>,
    #[account(
        mut,
        seeds = [CAMPAIGN_VAULT_SEED, &campaign_id.to_le_bytes()],
        bump = campaign.vault_bump,
    )]
    pub campaign_vault: Account<'info, TokenAccount>,
    #[account(seeds = [AGENT_SEED, &publisher_id.to_le_bytes()], bump = publisher_agent.bump)]
    pub publisher_agent: Account<'info, AgentInfo>,
    #[account(
        seeds = [AGENT_SEED, &validator_id.to_le_bytes()],
        bump = validator_agent.bump,
        constraint = validator_agent.authority == validator.key() @ ApexError::Unauthorized,
    )]
    pub validator_agent: Account<'info, AgentInfo>,
    #[account(
        mut,
        constraint = publisher_token_account.mint == campaign.budget_mint @ ApexError::Unauthorized,
        constraint = publisher_token_account.owner == publisher_agent.authority @ ApexError::InvalidAgentAuthority,
    )]
    pub publisher_token_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = validator,
        space = 8 + ProcessedAction::INIT_SPACE,
        seeds = [PROCESSED_ACTION_SEED, &campaign_id.to_le_bytes(), &action_hash],
        bump,
    )]
    pub processed_action: Account<'info, ProcessedAction>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> ProcessAction<'info> {
    fn transfer_to_publisher_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.campaign_vault.to_account_info(),
            to: self.publisher_token_account.to_account_info(),
            authority: self.campaign.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
#[instruction(campaign_id: u64)]
pub struct WithdrawRemainingBudget<'info> {
    #[account(mut)]
    pub advertiser: Signer<'info>,
    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, &campaign_id.to_le_bytes()],
        bump = campaign.bump,
    )]
    pub campaign: Account<'info, CampaignAccount>,
    #[account(
        mut,
        seeds = [CAMPAIGN_VAULT_SEED, &campaign_id.to_le_bytes()],
        bump = campaign.vault_bump,
    )]
    pub campaign_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = advertiser_token_account.owner == advertiser.key(),
        constraint = advertiser_token_account.mint == campaign.budget_mint,
    )]
    pub advertiser_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

impl<'info> WithdrawRemainingBudget<'info> {
    fn withdraw_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.campaign_vault.to_account_info(),
            to: self.advertiser_token_account.to_account_info(),
            authority: self.campaign.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
#[instruction(next_ad_id: u64, campaign_id: u64, publisher_id: u64, _start_time: i64, _metadata: Vec<u8>)]
pub struct CreateAd<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(mut, seeds = [STATE_SEED], bump = state.bump)]
    pub state: Account<'info, GlobalState>,
    #[account(
        seeds = [CAMPAIGN_SEED, &campaign_id.to_le_bytes()],
        bump = campaign.bump,
    )]
    pub campaign: Account<'info, CampaignAccount>,
    #[account(
        seeds = [AGENT_SEED, &campaign.advertiser_id.to_le_bytes()],
        bump = advertiser_agent.bump,
    )]
    pub advertiser_agent: Account<'info, AgentInfo>,
    #[account(seeds = [AGENT_SEED, &publisher_id.to_le_bytes()], bump = publisher_agent.bump)]
    pub publisher_agent: Account<'info, AgentInfo>,
    #[account(
        init,
        payer = caller,
        space = 8 + AdAccount::INIT_SPACE,
        seeds = [AD_SEED, &next_ad_id.to_le_bytes()],
        bump,
    )]
    pub ad: Account<'info, AdAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(ad_id: u64, _metadata: Vec<u8>)]
pub struct UpdateAd<'info> {
    #[account(mut)]
    pub advertiser: Signer<'info>,
    #[account(
        mut,
        seeds = [AD_SEED, &ad_id.to_le_bytes()],
        bump = ad.bump,
    )]
    pub ad: Account<'info, AdAccount>,
    #[account(
        seeds = [CAMPAIGN_SEED, &ad.campaign_id.to_le_bytes()],
        bump = campaign.bump,
    )]
    pub campaign: Account<'info, CampaignAccount>,
    #[account(
        seeds = [AGENT_SEED, &campaign.advertiser_id.to_le_bytes()],
        bump = advertiser_agent.bump,
    )]
    pub advertiser_agent: Account<'info, AgentInfo>,
}

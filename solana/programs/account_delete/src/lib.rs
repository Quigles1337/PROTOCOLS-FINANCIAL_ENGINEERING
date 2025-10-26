use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;

declare_id!("AccountDeleteXXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod account_delete {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.config.authority = ctx.accounts.authority.key();
        ctx.accounts.config.total_accounts = 0;
        ctx.accounts.config.bump = *ctx.bumps.get("config").unwrap();
        msg!("AccountDelete initialized");
        Ok(())
    }

    pub fn create_account(ctx: Context<CreateAccount>, beneficiary: Pubkey) -> Result<()> {
        let account_info = &mut ctx.accounts.account_info;
        let config = &mut ctx.accounts.config;

        account_info.owner = ctx.accounts.owner.key();
        account_info.beneficiary = beneficiary;
        account_info.active = true;
        account_info.bump = *ctx.bumps.get("account_info").unwrap();

        config.total_accounts += 1;

        emit!(AccountCreated {
            owner: account_info.owner,
            beneficiary,
        });

        Ok(())
    }

    pub fn set_beneficiary(ctx: Context<UpdateAccount>, new_beneficiary: Pubkey) -> Result<()> {
        let account_info = &mut ctx.accounts.account_info;

        require!(account_info.active, AccountDeleteError::NotActive);

        account_info.beneficiary = new_beneficiary;

        emit!(BeneficiaryUpdated {
            owner: account_info.owner,
            beneficiary: new_beneficiary,
        });

        Ok(())
    }

    pub fn request_deletion(ctx: Context<RequestDeletion>) -> Result<()> {
        let deletion = &mut ctx.accounts.deletion_request;
        let clock = Clock::get()?;

        deletion.owner = ctx.accounts.owner.key();
        deletion.grace_period_end = clock.unix_timestamp + 86400; // 24 hours grace period
        deletion.executed = false;
        deletion.bump = *ctx.bumps.get("deletion_request").unwrap();

        emit!(DeletionRequested {
            owner: deletion.owner,
            grace_period_end: deletion.grace_period_end,
        });

        Ok(())
    }

    pub fn execute_deletion(ctx: Context<ExecuteDeletion>) -> Result<()> {
        let deletion = &mut ctx.accounts.deletion_request;
        let account_info = &mut ctx.accounts.account_info;
        let clock = Clock::get()?;

        require!(!deletion.executed, AccountDeleteError::AlreadyExecuted);
        require!(
            clock.unix_timestamp >= deletion.grace_period_end,
            AccountDeleteError::GracePeriodNotEnded
        );
        require!(account_info.active, AccountDeleteError::NotActive);

        deletion.executed = true;
        account_info.active = false;

        emit!(AccountDeleted {
            owner: deletion.owner,
            beneficiary: account_info.beneficiary,
        });

        Ok(())
    }

    pub fn cancel_deletion(ctx: Context<CancelDeletion>) -> Result<()> {
        let deletion = &ctx.accounts.deletion_request;

        require!(!deletion.executed, AccountDeleteError::AlreadyExecuted);

        emit!(DeletionCancelled {
            owner: deletion.owner,
        });

        Ok(())
    }

    pub fn check_status(ctx: Context<CheckStatus>) -> Result<bool> {
        Ok(ctx.accounts.account_info.active)
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateAccount<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + AccountInfo::INIT_SPACE,
        seeds = [b"account", owner.key().as_ref()],
        bump
    )]
    pub account_info: Account<'info, AccountInfo>,
    
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateAccount<'info> {
    #[account(
        mut,
        seeds = [b"account", owner.key().as_ref()],
        bump = account_info.bump,
        has_one = owner
    )]
    pub account_info: Account<'info, AccountInfo>,
    
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct RequestDeletion<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + DeletionRequest::INIT_SPACE,
        seeds = [b"deletion", owner.key().as_ref()],
        bump
    )]
    pub deletion_request: Account<'info, DeletionRequest>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteDeletion<'info> {
    #[account(
        mut,
        seeds = [b"deletion", owner.key().as_ref()],
        bump = deletion_request.bump,
        has_one = owner
    )]
    pub deletion_request: Account<'info, DeletionRequest>,
    
    #[account(
        mut,
        seeds = [b"account", owner.key().as_ref()],
        bump = account_info.bump,
        has_one = owner
    )]
    pub account_info: Account<'info, AccountInfo>,
    
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelDeletion<'info> {
    #[account(
        mut,
        seeds = [b"deletion", owner.key().as_ref()],
        bump = deletion_request.bump,
        has_one = owner,
        close = owner
    )]
    pub deletion_request: Account<'info, DeletionRequest>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct CheckStatus<'info> {
    pub account_info: Account<'info, AccountInfo>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub total_accounts: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AccountInfo {
    pub owner: Pubkey,
    pub beneficiary: Pubkey,
    pub active: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct DeletionRequest {
    pub owner: Pubkey,
    pub grace_period_end: i64,
    pub executed: bool,
    pub bump: u8,
}

#[event]
pub struct AccountCreated {
    pub owner: Pubkey,
    pub beneficiary: Pubkey,
}

#[event]
pub struct BeneficiaryUpdated {
    pub owner: Pubkey,
    pub beneficiary: Pubkey,
}

#[event]
pub struct DeletionRequested {
    pub owner: Pubkey,
    pub grace_period_end: i64,
}

#[event]
pub struct AccountDeleted {
    pub owner: Pubkey,
    pub beneficiary: Pubkey,
}

#[event]
pub struct DeletionCancelled {
    pub owner: Pubkey,
}

#[error_code]
pub enum AccountDeleteError {
    #[msg("Deletion already executed")]
    AlreadyExecuted,
    #[msg("Grace period has not ended")]
    GracePeriodNotEnded,
    #[msg("Account is not active")]
    NotActive,
}

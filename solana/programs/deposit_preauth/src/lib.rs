use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;

declare_id!("DepositPreauthXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod deposit_preauth {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.config.authority = ctx.accounts.authority.key();
        ctx.accounts.config.total_preauths = 0;
        ctx.accounts.config.bump = *ctx.bumps.get("config").unwrap();
        msg!("DepositPreauth initialized");
        Ok(())
    }

    pub fn create_preauth(
        ctx: Context<CreatePreauth>,
        authorized: Pubkey,
        max_amount: u64,
        expiration: i64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        require!(expiration > clock.unix_timestamp, PreauthError::InvalidExpiration);
        require!(max_amount > 0, PreauthError::InvalidAmount);

        let preauth = &mut ctx.accounts.preauth;
        let config = &mut ctx.accounts.config;

        preauth.authorizer = ctx.accounts.authorizer.key();
        preauth.authorized = authorized;
        preauth.max_amount = max_amount;
        preauth.used = false;
        preauth.expiration = expiration;
        preauth.bump = *ctx.bumps.get("preauth").unwrap();

        config.total_preauths += 1;

        emit!(PreauthCreated {
            authorizer: preauth.authorizer,
            authorized,
            max_amount,
        });

        Ok(())
    }

    pub fn use_preauth(ctx: Context<UsePreauth>, amount: u64) -> Result<()> {
        let preauth = &mut ctx.accounts.preauth;
        let clock = Clock::get()?;

        require!(!preauth.used, PreauthError::AlreadyUsed);
        require!(clock.unix_timestamp < preauth.expiration, PreauthError::Expired);
        require!(amount <= preauth.max_amount, PreauthError::ExceedsMax);
        require!(amount > 0, PreauthError::InvalidAmount);

        preauth.used = true;

        emit!(PreauthUsed {
            authorizer: preauth.authorizer,
            authorized: preauth.authorized,
            amount,
        });

        Ok(())
    }

    pub fn revoke_preauth(ctx: Context<RevokePreauth>) -> Result<()> {
        let preauth = &ctx.accounts.preauth;

        require!(!preauth.used, PreauthError::AlreadyUsed);

        emit!(PreauthRevoked {
            authorizer: preauth.authorizer,
            authorized: preauth.authorized,
        });

        Ok(())
    }

    pub fn check_preauth(ctx: Context<CheckPreauth>, amount: u64) -> Result<bool> {
        let preauth = &ctx.accounts.preauth;
        let clock = Clock::get()?;

        Ok(
            !preauth.used
                && clock.unix_timestamp < preauth.expiration
                && amount <= preauth.max_amount
        )
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
pub struct CreatePreauth<'info> {
    #[account(
        init,
        payer = authorizer,
        space = 8 + Preauth::INIT_SPACE,
        seeds = [b"preauth", authorizer.key().as_ref(), config.total_preauths.to_le_bytes().as_ref()],
        bump
    )]
    pub preauth: Account<'info, Preauth>,
    
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,
    
    #[account(mut)]
    pub authorizer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UsePreauth<'info> {
    #[account(
        mut,
        constraint = preauth.authorized == authorized.key()
    )]
    pub preauth: Account<'info, Preauth>,
    
    pub authorized: Signer<'info>,
}

#[derive(Accounts)]
pub struct RevokePreauth<'info> {
    #[account(
        mut,
        has_one = authorizer,
        close = authorizer
    )]
    pub preauth: Account<'info, Preauth>,
    
    #[account(mut)]
    pub authorizer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CheckPreauth<'info> {
    pub preauth: Account<'info, Preauth>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub total_preauths: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Preauth {
    pub authorizer: Pubkey,
    pub authorized: Pubkey,
    pub max_amount: u64,
    pub used: bool,
    pub expiration: i64,
    pub bump: u8,
}

#[event]
pub struct PreauthCreated {
    pub authorizer: Pubkey,
    pub authorized: Pubkey,
    pub max_amount: u64,
}

#[event]
pub struct PreauthUsed {
    pub authorizer: Pubkey,
    pub authorized: Pubkey,
    pub amount: u64,
}

#[event]
pub struct PreauthRevoked {
    pub authorizer: Pubkey,
    pub authorized: Pubkey,
}

#[error_code]
pub enum PreauthError {
    #[msg("Invalid expiration timestamp")]
    InvalidExpiration,
    #[msg("Preauth already used")]
    AlreadyUsed,
    #[msg("Preauth expired")]
    Expired,
    #[msg("Amount exceeds maximum")]
    ExceedsMax,
    #[msg("Invalid amount")]
    InvalidAmount,
}

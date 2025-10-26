use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;

declare_id!("DepositAuthXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod deposit_authorization {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, compliance_officer: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.compliance_officer = compliance_officer;
        config.total_auths = 0;
        config.bump = *ctx.bumps.get("config").unwrap();
        msg!("DepositAuthorization initialized");
        Ok(())
    }

    pub fn create_authorization(
        ctx: Context<CreateAuth>,
        authorized: Pubkey,
        asset_id: u32,
        max_amount: u64,
        expiration: i64,
        tier: AuthTier,
    ) -> Result<()> {
        let clock = Clock::get()?;
        require!(expiration > clock.unix_timestamp, AuthError::InvalidExpiration);

        let auth = &mut ctx.accounts.authorization;
        let config = &mut ctx.accounts.config;

        auth.authorizer = ctx.accounts.authorizer.key();
        auth.authorized = authorized;
        auth.asset_id = asset_id;
        auth.max_amount = max_amount;
        auth.expiration = expiration;
        auth.tier = tier.clone();
        auth.status = AuthStatus::Active;
        auth.bump = *ctx.bumps.get("authorization").unwrap();

        config.total_auths += 1;

        emit!(AuthCreated {
            authorizer: auth.authorizer,
            authorized,
            asset_id,
            tier: auth.tier.clone(),
        });

        Ok(())
    }

    pub fn revoke_authorization(ctx: Context<RevokeAuth>) -> Result<()> {
        let auth = &mut ctx.accounts.authorization;
        
        require!(matches!(auth.status, AuthStatus::Active), AuthError::NotActive);
        
        auth.status = AuthStatus::Revoked;

        emit!(AuthRevoked {
            authorizer: auth.authorizer,
            authorized: auth.authorized,
            asset_id: auth.asset_id,
        });

        Ok(())
    }

    pub fn update_tier(ctx: Context<UpdateAuth>, new_tier: AuthTier) -> Result<()> {
        let auth = &mut ctx.accounts.authorization;
        
        require!(matches!(auth.status, AuthStatus::Active), AuthError::NotActive);
        
        auth.tier = new_tier.clone();

        emit!(TierUpdated {
            authorizer: auth.authorizer,
            authorized: auth.authorized,
            new_tier,
        });

        Ok(())
    }

    pub fn check_authorization(
        ctx: Context<CheckAuth>,
        amount: u64,
    ) -> Result<bool> {
        let auth = &ctx.accounts.authorization;
        let clock = Clock::get()?;

        Ok(
            matches!(auth.status, AuthStatus::Active)
                && clock.unix_timestamp < auth.expiration
                && amount <= auth.max_amount
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
pub struct CreateAuth<'info> {
    #[account(
        init,
        payer = authorizer,
        space = 8 + Authorization::INIT_SPACE,
        seeds = [b"auth", authorizer.key().as_ref(), config.total_auths.to_le_bytes().as_ref()],
        bump
    )]
    pub authorization: Account<'info, Authorization>,
    
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
pub struct RevokeAuth<'info> {
    #[account(
        mut,
        has_one = authorizer
    )]
    pub authorization: Account<'info, Authorization>,
    
    pub authorizer: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateAuth<'info> {
    #[account(
        mut,
        has_one = authorizer
    )]
    pub authorization: Account<'info, Authorization>,
    
    pub authorizer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CheckAuth<'info> {
    pub authorization: Account<'info, Authorization>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub compliance_officer: Pubkey,
    pub total_auths: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Authorization {
    pub authorizer: Pubkey,
    pub authorized: Pubkey,
    pub asset_id: u32,
    pub max_amount: u64,
    pub expiration: i64,
    pub tier: AuthTier,
    pub status: AuthStatus,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum AuthTier {
    Basic,
    Standard,
    Premium,
    Institutional,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum AuthStatus {
    Active,
    Suspended,
    Revoked,
}

#[event]
pub struct AuthCreated {
    pub authorizer: Pubkey,
    pub authorized: Pubkey,
    pub asset_id: u32,
    pub tier: AuthTier,
}

#[event]
pub struct AuthRevoked {
    pub authorizer: Pubkey,
    pub authorized: Pubkey,
    pub asset_id: u32,
}

#[event]
pub struct TierUpdated {
    pub authorizer: Pubkey,
    pub authorized: Pubkey,
    pub new_tier: AuthTier,
}

#[error_code]
pub enum AuthError {
    #[msg("Invalid expiration timestamp")]
    InvalidExpiration,
    #[msg("Authorization not active")]
    NotActive,
}

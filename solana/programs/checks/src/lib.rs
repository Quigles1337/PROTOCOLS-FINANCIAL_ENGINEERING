use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;

declare_id!("ChecksXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod checks {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.config.authority = ctx.accounts.authority.key();
        ctx.accounts.config.total_checks = 0;
        ctx.accounts.config.bump = *ctx.bumps.get("config").unwrap();
        Ok(())
    }

    pub fn create_check(ctx: Context<CreateCheck>, amount: u64, expiration: i64, memo: [u8; 32]) -> Result<()> {
        let check = &mut ctx.accounts.check;
        let clock = Clock::get()?;
        require!(expiration > clock.unix_timestamp, CheckError::InvalidExpiration);
        check.drawer = ctx.accounts.drawer.key();
        check.payee = ctx.accounts.payee.key();
        check.amount = amount;
        check.expiration = expiration;
        check.status = CheckStatus::Active;
        check.memo = memo;
        check.bump = *ctx.bumps.get("check").unwrap();
        ctx.accounts.config.total_checks += 1;
        emit!(CheckCreated { drawer: check.drawer, payee: check.payee, amount });
        Ok(())
    }

    pub fn cash_check(ctx: Context<CashCheck>) -> Result<()> {
        let check = &mut ctx.accounts.check;
        let clock = Clock::get()?;
        require!(matches!(check.status, CheckStatus::Active), CheckError::NotActive);
        require!(clock.unix_timestamp < check.expiration, CheckError::Expired);
        check.status = CheckStatus::Cashed;
        emit!(CheckCashed { drawer: check.drawer, payee: check.payee });
        Ok(())
    }

    pub fn cancel_check(ctx: Context<CancelCheck>) -> Result<()> {
        let check = &mut ctx.accounts.check;
        require!(matches!(check.status, CheckStatus::Active), CheckError::NotActive);
        check.status = CheckStatus::Cancelled;
        emit!(CheckCancelled { drawer: check.drawer, payee: check.payee });
        Ok(())
    }

    pub fn endorse_check(ctx: Context<EndorseCheck>, new_payee: Pubkey) -> Result<()> {
        let check = &mut ctx.accounts.check;
        require!(matches!(check.status, CheckStatus::Active), CheckError::NotActive);
        check.payee = new_payee;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + Config::INIT_SPACE, seeds = [b"config"], bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateCheck<'info> {
    #[account(init, payer = drawer, space = 8 + Check::INIT_SPACE, seeds = [b"check", drawer.key().as_ref(), &drawer.key().to_bytes()[..8]], bump)]
    pub check: Account<'info, Check>,
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub drawer: Signer<'info>,
    /// CHECK: Payee
    pub payee: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CashCheck<'info> {
    #[account(mut, close = payee, constraint = check.payee == payee.key())]
    pub check: Account<'info, Check>,
    #[account(mut)]
    pub payee: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelCheck<'info> {
    #[account(mut, has_one = drawer, close = drawer)]
    pub check: Account<'info, Check>,
    #[account(mut)]
    pub drawer: Signer<'info>,
}

#[derive(Accounts)]
pub struct EndorseCheck<'info> {
    #[account(mut, constraint = check.payee == current_payee.key())]
    pub check: Account<'info, Check>,
    pub current_payee: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub total_checks: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Check {
    pub drawer: Pubkey,
    pub payee: Pubkey,
    pub amount: u64,
    pub expiration: i64,
    pub status: CheckStatus,
    pub memo: [u8; 32],
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum CheckStatus {
    Active,
    Cashed,
    Cancelled,
}

#[event]
pub struct CheckCreated {
    pub drawer: Pubkey,
    pub payee: Pubkey,
    pub amount: u64,
}

#[event]
pub struct CheckCashed {
    pub drawer: Pubkey,
    pub payee: Pubkey,
}

#[event]
pub struct CheckCancelled {
    pub drawer: Pubkey,
    pub payee: Pubkey,
}

#[error_code]
pub enum CheckError {
    #[msg("Invalid expiration")]
    InvalidExpiration,
    #[msg("Not active")]
    NotActive,
    #[msg("Expired")]
    Expired,
}

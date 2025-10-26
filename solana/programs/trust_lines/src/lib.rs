use anchor_lang::prelude::*;

declare_id!("TrustL1nesXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod trust_lines {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.total_lines = 0;
        config.bump = *ctx.bumps.get("config").unwrap();
        msg!("TrustLines program initialized");
        Ok(())
    }

    pub fn create_trust_line(ctx: Context<CreateTrustLine>, limit: u64, quality_in: u32, quality_out: u32) -> Result<()> {
        require!(quality_in <= 100, TrustLineError::InvalidQuality);
        require!(quality_out <= 100, TrustLineError::InvalidQuality);
        let trust_line = &mut ctx.accounts.trust_line;
        let config = &mut ctx.accounts.config;
        trust_line.owner = ctx.accounts.owner.key();
        trust_line.counterparty = ctx.accounts.counterparty.key();
        trust_line.limit = limit;
        trust_line.balance = 0;
        trust_line.quality_in = quality_in;
        trust_line.quality_out = quality_out;
        trust_line.authorized = true;
        trust_line.bump = *ctx.bumps.get("trust_line").unwrap();
        config.total_lines += 1;
        emit!(TrustLineCreated { owner: trust_line.owner, counterparty: trust_line.counterparty, limit });
        Ok(())
    }

    pub fn send_payment(ctx: Context<SendPayment>, amount: u64) -> Result<()> {
        let trust_line = &mut ctx.accounts.trust_line;
        require!(trust_line.authorized, TrustLineError::NotAuthorized);
        let signed_amount = amount as i64;
        require!(trust_line.balance + signed_amount <= trust_line.limit as i64, TrustLineError::LimitExceeded);
        trust_line.balance += signed_amount;
        emit!(PaymentSent { from: trust_line.owner, to: trust_line.counterparty, amount, new_balance: trust_line.balance });
        Ok(())
    }

    pub fn close_trust_line(ctx: Context<CloseTrustLine>) -> Result<()> {
        let trust_line = &ctx.accounts.trust_line;
        require!(trust_line.balance == 0, TrustLineError::NonZeroBalance);
        let config = &mut ctx.accounts.config;
        config.total_lines -= 1;
        emit!(TrustLineClosed { owner: trust_line.owner, counterparty: trust_line.counterparty });
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
pub struct CreateTrustLine<'info> {
    #[account(init, payer = owner, space = 8 + TrustLine::INIT_SPACE, seeds = [b"trust_line", owner.key().as_ref(), counterparty.key().as_ref()], bump)]
    pub trust_line: Account<'info, TrustLine>,
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: Counterparty
    pub counterparty: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SendPayment<'info> {
    #[account(mut, seeds = [b"trust_line", sender.key().as_ref(), trust_line.counterparty.as_ref()], bump = trust_line.bump, constraint = trust_line.owner == sender.key())]
    pub trust_line: Account<'info, TrustLine>,
    pub sender: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseTrustLine<'info> {
    #[account(mut, seeds = [b"trust_line", owner.key().as_ref(), trust_line.counterparty.as_ref()], bump = trust_line.bump, has_one = owner, close = owner)]
    pub trust_line: Account<'info, TrustLine>,
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub owner: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub total_lines: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct TrustLine {
    pub owner: Pubkey,
    pub counterparty: Pubkey,
    pub limit: u64,
    pub balance: i64,
    pub quality_in: u32,
    pub quality_out: u32,
    pub authorized: bool,
    pub bump: u8,
}

#[event]
pub struct TrustLineCreated {
    pub owner: Pubkey,
    pub counterparty: Pubkey,
    pub limit: u64,
}

#[event]
pub struct PaymentSent {
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
    pub new_balance: i64,
}

#[event]
pub struct TrustLineClosed {
    pub owner: Pubkey,
    pub counterparty: Pubkey,
}

#[error_code]
pub enum TrustLineError {
    #[msg("Quality must be <= 100")]
    InvalidQuality,
    #[msg("Not authorized")]
    NotAuthorized,
    #[msg("Limit exceeded")]
    LimitExceeded,
    #[msg("Non-zero balance")]
    NonZeroBalance,
}

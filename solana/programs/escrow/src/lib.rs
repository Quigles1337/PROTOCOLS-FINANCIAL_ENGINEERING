use anchor_lang::prelude::*;
use anchor_lang::solana_program::{clock::Clock, hash::hash};

declare_id!("EscrowXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod escrow {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.total_escrows = 0;
        config.bump = *ctx.bumps.get("config").unwrap();
        Ok(())
    }

    pub fn create_escrow(ctx: Context<CreateEscrow>, amount: u64, hash_lock: [u8; 32], time_lock: i64) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let clock = Clock::get()?;
        require!(time_lock > clock.unix_timestamp, EscrowError::InvalidTimeLock);
        escrow.sender = ctx.accounts.sender.key();
        escrow.recipient = ctx.accounts.recipient.key();
        escrow.amount = amount;
        escrow.hash_lock = hash_lock;
        escrow.time_lock = time_lock;
        escrow.status = EscrowStatus::Active;
        escrow.bump = *ctx.bumps.get("escrow").unwrap();
        let config = &mut ctx.accounts.config;
        config.total_escrows += 1;
        emit!(EscrowCreated { sender: escrow.sender, recipient: escrow.recipient, amount });
        Ok(())
    }

    pub fn complete_escrow(ctx: Context<CompleteEscrow>, preimage: [u8; 32]) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(matches!(escrow.status, EscrowStatus::Active), EscrowError::NotActive);
        let computed_hash = hash(&preimage).to_bytes();
        require!(computed_hash == escrow.hash_lock, EscrowError::InvalidPreimage);
        escrow.status = EscrowStatus::Completed;
        emit!(EscrowCompleted { sender: escrow.sender, recipient: escrow.recipient });
        Ok(())
    }

    pub fn refund_escrow(ctx: Context<RefundEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let clock = Clock::get()?;
        require!(matches!(escrow.status, EscrowStatus::Active), EscrowError::NotActive);
        require!(clock.unix_timestamp >= escrow.time_lock, EscrowError::TimeLockNotExpired);
        escrow.status = EscrowStatus::Refunded;
        emit!(EscrowRefunded { sender: escrow.sender, recipient: escrow.recipient });
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
pub struct CreateEscrow<'info> {
    #[account(init, payer = sender, space = 8 + Escrow::INIT_SPACE, seeds = [b"escrow", sender.key().as_ref(), recipient.key().as_ref()], bump)]
    pub escrow: Account<'info, Escrow>,
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub sender: Signer<'info>,
    /// CHECK: Recipient
    pub recipient: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CompleteEscrow<'info> {
    #[account(mut, seeds = [b"escrow", escrow.sender.as_ref(), recipient.key().as_ref()], bump = escrow.bump, has_one = recipient, close = recipient)]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub recipient: Signer<'info>,
}

#[derive(Accounts)]
pub struct RefundEscrow<'info> {
    #[account(mut, seeds = [b"escrow", sender.key().as_ref(), escrow.recipient.as_ref()], bump = escrow.bump, has_one = sender, close = sender)]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub sender: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub total_escrows: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
    pub hash_lock: [u8; 32],
    pub time_lock: i64,
    pub status: EscrowStatus,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum EscrowStatus {
    Active,
    Completed,
    Refunded,
}

#[event]
pub struct EscrowCreated {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
}

#[event]
pub struct EscrowCompleted {
    pub sender: Pubkey,
    pub recipient: Pubkey,
}

#[event]
pub struct EscrowRefunded {
    pub sender: Pubkey,
    pub recipient: Pubkey,
}

#[error_code]
pub enum EscrowError {
    #[msg("Invalid time lock")]
    InvalidTimeLock,
    #[msg("Not active")]
    NotActive,
    #[msg("Invalid preimage")]
    InvalidPreimage,
    #[msg("Time lock not expired")]
    TimeLockNotExpired,
}

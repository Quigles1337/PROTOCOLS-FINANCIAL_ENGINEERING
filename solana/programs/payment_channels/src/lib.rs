use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;

declare_id!("PaymentChannelsXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod payment_channels {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.total_channels = 0;
        config.bump = *ctx.bumps.get("config").unwrap();
        Ok(())
    }

    pub fn open_channel(ctx: Context<OpenChannel>, amount: u64, expiration: i64) -> Result<()> {
        let channel = &mut ctx.accounts.channel;
        let clock = Clock::get()?;
        require!(expiration > clock.unix_timestamp, ChannelError::InvalidExpiration);
        channel.participant_a = ctx.accounts.participant_a.key();
        channel.participant_b = ctx.accounts.participant_b.key();
        channel.balance_a = amount;
        channel.balance_b = 0;
        channel.nonce = 0;
        channel.status = ChannelStatus::Open;
        channel.expiration = expiration;
        channel.dispute_expiration = 0;
        channel.bump = *ctx.bumps.get("channel").unwrap();
        let config = &mut ctx.accounts.config;
        config.total_channels += 1;
        emit!(ChannelOpened { participant_a: channel.participant_a, participant_b: channel.participant_b, balance_a: amount });
        Ok(())
    }

    pub fn fund_channel(ctx: Context<FundChannel>, amount: u64) -> Result<()> {
        let channel = &mut ctx.accounts.channel;
        require!(matches!(channel.status, ChannelStatus::Open), ChannelError::ChannelNotOpen);
        channel.balance_b += amount;
        Ok(())
    }

    pub fn cooperative_close(ctx: Context<CooperativeClose>, final_balance_a: u64, final_balance_b: u64) -> Result<()> {
        let channel = &mut ctx.accounts.channel;
        require!(matches!(channel.status, ChannelStatus::Open), ChannelError::ChannelNotOpen);
        require!(final_balance_a + final_balance_b == channel.balance_a + channel.balance_b, ChannelError::InvalidBalances);
        channel.status = ChannelStatus::Closed;
        emit!(ChannelClosed { participant_a: channel.participant_a, participant_b: channel.participant_b });
        Ok(())
    }

    pub fn raise_dispute(ctx: Context<RaiseDispute>, nonce: u64, balance_a: u64, balance_b: u64) -> Result<()> {
        let channel = &mut ctx.accounts.channel;
        let clock = Clock::get()?;
        require!(matches!(channel.status, ChannelStatus::Open), ChannelError::ChannelNotOpen);
        require!(nonce > channel.nonce, ChannelError::InvalidNonce);
        require!(balance_a + balance_b == channel.balance_a + channel.balance_b, ChannelError::InvalidBalances);
        channel.status = ChannelStatus::InDispute;
        channel.nonce = nonce;
        channel.balance_a = balance_a;
        channel.balance_b = balance_b;
        channel.dispute_expiration = clock.unix_timestamp + 86400;
        emit!(DisputeRaised { participant_a: channel.participant_a, participant_b: channel.participant_b });
        Ok(())
    }

    pub fn settle_dispute(ctx: Context<SettleDispute>) -> Result<()> {
        let channel = &mut ctx.accounts.channel;
        let clock = Clock::get()?;
        require!(matches!(channel.status, ChannelStatus::InDispute), ChannelError::NoDispute);
        require!(clock.unix_timestamp >= channel.dispute_expiration, ChannelError::DisputePeriodNotOver);
        channel.status = ChannelStatus::Closed;
        emit!(ChannelClosed { participant_a: channel.participant_a, participant_b: channel.participant_b });
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
pub struct OpenChannel<'info> {
    #[account(init, payer = participant_a, space = 8 + Channel::INIT_SPACE, seeds = [b"channel", participant_a.key().as_ref(), participant_b.key().as_ref()], bump)]
    pub channel: Account<'info, Channel>,
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub participant_a: Signer<'info>,
    /// CHECK: Participant B
    pub participant_b: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FundChannel<'info> {
    #[account(mut, seeds = [b"channel", channel.participant_a.as_ref(), participant_b.key().as_ref()], bump = channel.bump, constraint = channel.participant_b == participant_b.key())]
    pub channel: Account<'info, Channel>,
    #[account(mut)]
    pub participant_b: Signer<'info>,
}

#[derive(Accounts)]
pub struct CooperativeClose<'info> {
    #[account(mut, seeds = [b"channel", channel.participant_a.as_ref(), channel.participant_b.as_ref()], bump = channel.bump, close = participant)]
    pub channel: Account<'info, Channel>,
    #[account(mut)]
    pub participant: Signer<'info>,
}

#[derive(Accounts)]
pub struct RaiseDispute<'info> {
    #[account(mut, seeds = [b"channel", channel.participant_a.as_ref(), channel.participant_b.as_ref()], bump = channel.bump)]
    pub channel: Account<'info, Channel>,
    pub participant: Signer<'info>,
}

#[derive(Accounts)]
pub struct SettleDispute<'info> {
    #[account(mut, seeds = [b"channel", channel.participant_a.as_ref(), channel.participant_b.as_ref()], bump = channel.bump, close = participant_a)]
    pub channel: Account<'info, Channel>,
    /// CHECK: Participant A
    #[account(mut)]
    pub participant_a: AccountInfo<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub total_channels: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Channel {
    pub participant_a: Pubkey,
    pub participant_b: Pubkey,
    pub balance_a: u64,
    pub balance_b: u64,
    pub nonce: u64,
    pub status: ChannelStatus,
    pub expiration: i64,
    pub dispute_expiration: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum ChannelStatus {
    Open,
    InDispute,
    Closed,
}

#[event]
pub struct ChannelOpened {
    pub participant_a: Pubkey,
    pub participant_b: Pubkey,
    pub balance_a: u64,
}

#[event]
pub struct DisputeRaised {
    pub participant_a: Pubkey,
    pub participant_b: Pubkey,
}

#[event]
pub struct ChannelClosed {
    pub participant_a: Pubkey,
    pub participant_b: Pubkey,
}

#[error_code]
pub enum ChannelError {
    #[msg("Invalid expiration")]
    InvalidExpiration,
    #[msg("Channel not open")]
    ChannelNotOpen,
    #[msg("Invalid balances")]
    InvalidBalances,
    #[msg("Invalid nonce")]
    InvalidNonce,
    #[msg("No dispute")]
    NoDispute,
    #[msg("Dispute period not over")]
    DisputePeriodNotOver,
}

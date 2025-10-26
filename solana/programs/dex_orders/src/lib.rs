use anchor_lang::prelude::*;

declare_id!("DEXOrdersXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod dex_orders {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.total_orders = 0;
        config.bump = *ctx.bumps.get("config").unwrap();
        msg!("DEXOrders initialized");
        Ok(())
    }

    pub fn place_order(
        ctx: Context<PlaceOrder>,
        sell_asset: u32,
        buy_asset: u32,
        sell_amount: u64,
        buy_amount: u64,
    ) -> Result<()> {
        require!(sell_asset != buy_asset, OrderError::SameAsset);
        require!(sell_amount > 0, OrderError::InvalidAmount);
        require!(buy_amount > 0, OrderError::InvalidAmount);

        let order = &mut ctx.accounts.order;
        let config = &mut ctx.accounts.config;

        order.maker = ctx.accounts.maker.key();
        order.sell_asset = sell_asset;
        order.buy_asset = buy_asset;
        order.sell_amount = sell_amount;
        order.buy_amount = buy_amount;
        order.filled = 0;
        order.status = OrderStatus::Open;
        order.bump = *ctx.bumps.get("order").unwrap();

        config.total_orders += 1;

        emit!(OrderPlaced {
            maker: order.maker,
            order_id: order.key(),
            sell_asset,
            buy_asset,
            sell_amount,
            buy_amount,
        });

        Ok(())
    }

    pub fn fill_order(ctx: Context<FillOrder>, fill_amount: u64) -> Result<()> {
        let order = &mut ctx.accounts.order;
        
        require!(matches!(order.status, OrderStatus::Open | OrderStatus::PartiallyFilled), OrderError::NotOpen);
        require!(fill_amount > 0, OrderError::InvalidAmount);

        let remaining = order.sell_amount - order.filled;
        let actual_fill = fill_amount.min(remaining);
        
        let required_payment = (actual_fill as u128 * order.buy_amount as u128 / order.sell_amount as u128) as u64;

        order.filled += actual_fill;

        if order.filled >= order.sell_amount {
            order.status = OrderStatus::Filled;
        } else {
            order.status = OrderStatus::PartiallyFilled;
        }

        emit!(OrderFilled {
            maker: order.maker,
            taker: ctx.accounts.taker.key(),
            amount: actual_fill,
            payment: required_payment,
        });

        Ok(())
    }

    pub fn cancel_order(ctx: Context<CancelOrder>) -> Result<()> {
        let order = &mut ctx.accounts.order;
        
        require!(matches!(order.status, OrderStatus::Open | OrderStatus::PartiallyFilled), OrderError::NotOpen);
        
        order.status = OrderStatus::Cancelled;

        emit!(OrderCancelled {
            maker: order.maker,
            order_id: order.key(),
        });

        Ok(())
    }

    pub fn get_price(ctx: Context<GetPrice>) -> Result<(u64, u64)> {
        let order = &ctx.accounts.order;
        Ok((order.buy_amount, order.sell_amount))
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
pub struct PlaceOrder<'info> {
    #[account(
        init,
        payer = maker,
        space = 8 + Order::INIT_SPACE,
        seeds = [b"order", maker.key().as_ref(), config.total_orders.to_le_bytes().as_ref()],
        bump
    )]
    pub order: Account<'info, Order>,
    
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,
    
    #[account(mut)]
    pub maker: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FillOrder<'info> {
    #[account(mut)]
    pub order: Account<'info, Order>,
    
    pub taker: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelOrder<'info> {
    #[account(
        mut,
        has_one = maker,
        close = maker
    )]
    pub order: Account<'info, Order>,
    
    #[account(mut)]
    pub maker: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetPrice<'info> {
    pub order: Account<'info, Order>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub total_orders: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Order {
    pub maker: Pubkey,
    pub sell_asset: u32,
    pub buy_asset: u32,
    pub sell_amount: u64,
    pub buy_amount: u64,
    pub filled: u64,
    pub status: OrderStatus,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}

#[event]
pub struct OrderPlaced {
    pub maker: Pubkey,
    pub order_id: Pubkey,
    pub sell_asset: u32,
    pub buy_asset: u32,
    pub sell_amount: u64,
    pub buy_amount: u64,
}

#[event]
pub struct OrderFilled {
    pub maker: Pubkey,
    pub taker: Pubkey,
    pub amount: u64,
    pub payment: u64,
}

#[event]
pub struct OrderCancelled {
    pub maker: Pubkey,
    pub order_id: Pubkey,
}

#[error_code]
pub enum OrderError {
    #[msg("Same asset specified for buy and sell")]
    SameAsset,
    #[msg("Order is not open")]
    NotOpen,
    #[msg("Invalid amount specified")]
    InvalidAmount,
}

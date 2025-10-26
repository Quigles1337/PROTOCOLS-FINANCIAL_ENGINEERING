// dex_orders - Production Solana Anchor program
use anchor_lang::prelude::*;
declare_id!("dex_ordersXXXXXXXXXXXXXXXXXXXX");
#[program]
pub mod dex_orders {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    pub authority: Signer<'info>,
}

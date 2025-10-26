// account_delete - Production Solana Anchor program
use anchor_lang::prelude::*;
declare_id!("account_deleteXXXXXXXXXXXXXXXXXXXX");
#[program]
pub mod account_delete {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    pub authority: Signer<'info>,
}

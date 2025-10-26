// deposit_authorization - Production Solana Anchor program
use anchor_lang::prelude::*;
declare_id!("deposit_authorizationXXXXXXXXXXXXXXXXXXXX");
#[program]
pub mod deposit_authorization {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    pub authority: Signer<'info>,
}

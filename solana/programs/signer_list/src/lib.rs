// signer_list - Production Solana Anchor program
use anchor_lang::prelude::*;
declare_id!("signer_listXXXXXXXXXXXXXXXXXXXX");
#[program]
pub mod signer_list {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    pub authority: Signer<'info>,
}

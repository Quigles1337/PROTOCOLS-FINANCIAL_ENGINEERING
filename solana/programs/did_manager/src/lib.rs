use anchor_lang::prelude::*;

declare_id!("DIDManagerXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod did_manager {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.config.authority = ctx.accounts.authority.key();
        ctx.accounts.config.total_dids = 0;
        ctx.accounts.config.bump = *ctx.bumps.get("config").unwrap();
        Ok(())
    }

    pub fn create_did(ctx: Context<CreateDID>, public_key: [u8; 32]) -> Result<()> {
        let did_doc = &mut ctx.accounts.did_document;
        did_doc.controller = ctx.accounts.controller.key();
        did_doc.public_key = public_key;
        did_doc.active = true;
        did_doc.bump = *ctx.bumps.get("did_document").unwrap();
        ctx.accounts.config.total_dids += 1;
        emit!(DIDCreated { controller: did_doc.controller });
        Ok(())
    }

    pub fn update_did(ctx: Context<UpdateDID>, new_public_key: [u8; 32]) -> Result<()> {
        let did_doc = &mut ctx.accounts.did_document;
        require!(did_doc.active, DIDError::NotActive);
        did_doc.public_key = new_public_key;
        emit!(DIDUpdated { controller: did_doc.controller });
        Ok(())
    }

    pub fn deactivate_did(ctx: Context<DeactivateDID>) -> Result<()> {
        let did_doc = &mut ctx.accounts.did_document;
        did_doc.active = false;
        emit!(DIDDeactivated { controller: did_doc.controller });
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
pub struct CreateDID<'info> {
    #[account(init, payer = controller, space = 8 + DIDDocument::INIT_SPACE, seeds = [b"did", controller.key().as_ref()], bump)]
    pub did_document: Account<'info, DIDDocument>,
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub controller: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateDID<'info> {
    #[account(mut, seeds = [b"did", controller.key().as_ref()], bump = did_document.bump, has_one = controller)]
    pub did_document: Account<'info, DIDDocument>,
    pub controller: Signer<'info>,
}

#[derive(Accounts)]
pub struct DeactivateDID<'info> {
    #[account(mut, seeds = [b"did", controller.key().as_ref()], bump = did_document.bump, has_one = controller)]
    pub did_document: Account<'info, DIDDocument>,
    pub controller: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub total_dids: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct DIDDocument {
    pub controller: Pubkey,
    pub public_key: [u8; 32],
    pub active: bool,
    pub bump: u8,
}

#[event]
pub struct DIDCreated {
    pub controller: Pubkey,
}

#[event]
pub struct DIDUpdated {
    pub controller: Pubkey,
}

#[event]
pub struct DIDDeactivated {
    pub controller: Pubkey,
}

#[error_code]
pub enum DIDError {
    #[msg("DID not active")]
    NotActive,
}

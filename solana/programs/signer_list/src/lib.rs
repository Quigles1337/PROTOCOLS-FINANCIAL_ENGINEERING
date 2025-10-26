use anchor_lang::prelude::*;

declare_id!("SignerListXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

#[program]
pub mod signer_list {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.config.authority = ctx.accounts.authority.key();
        ctx.accounts.config.total_lists = 0;
        ctx.accounts.config.bump = *ctx.bumps.get("config").unwrap();
        msg!("SignerList initialized");
        Ok(())
    }

    pub fn create_signer_list(ctx: Context<CreateSignerList>) -> Result<()> {
        let list = &mut ctx.accounts.signer_list;
        let config = &mut ctx.accounts.config;

        list.owner = ctx.accounts.owner.key();
        list.total_weight = 0;
        list.signer_count = 0;
        list.bump = *ctx.bumps.get("signer_list").unwrap();

        config.total_lists += 1;

        emit!(SignerListCreated {
            owner: list.owner,
        });

        Ok(())
    }

    pub fn add_signer(ctx: Context<AddSigner>, signer: Pubkey, weight: u32) -> Result<()> {
        require!(weight > 0, SignerListError::InvalidWeight);

        let list = &mut ctx.accounts.signer_list;

        list.total_weight += weight;
        list.signer_count += 1;

        emit!(SignerAdded {
            owner: list.owner,
            signer,
            weight,
        });

        Ok(())
    }

    pub fn remove_signer(ctx: Context<RemoveSigner>, weight: u32) -> Result<()> {
        let list = &mut ctx.accounts.signer_list;

        require!(list.total_weight >= weight, SignerListError::InsufficientWeight);
        require!(list.signer_count > 0, SignerListError::NoSigners);

        list.total_weight -= weight;
        list.signer_count -= 1;

        emit!(SignerRemoved {
            owner: list.owner,
            weight,
        });

        Ok(())
    }

    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        target: Pubkey,
        amount: u64,
    ) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;

        proposal.owner = ctx.accounts.owner.key();
        proposal.target = target;
        proposal.amount = amount;
        proposal.approvals_weight = 0;
        proposal.executed = false;
        proposal.bump = *ctx.bumps.get("proposal").unwrap();

        emit!(ProposalCreated {
            owner: proposal.owner,
            target,
            amount,
        });

        Ok(())
    }

    pub fn approve_proposal(ctx: Context<ApproveProposal>, weight: u32) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;

        require!(!proposal.executed, SignerListError::AlreadyExecuted);
        require!(weight > 0, SignerListError::InvalidWeight);

        proposal.approvals_weight += weight;

        emit!(ProposalApproved {
            owner: proposal.owner,
            approver: ctx.accounts.approver.key(),
            weight,
        });

        Ok(())
    }

    pub fn execute_proposal(ctx: Context<ExecuteProposal>, quorum: u32) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;

        require!(!proposal.executed, SignerListError::AlreadyExecuted);
        require!(proposal.approvals_weight >= quorum, SignerListError::QuorumNotMet);

        proposal.executed = true;

        emit!(ProposalExecuted {
            owner: proposal.owner,
            target: proposal.target,
            amount: proposal.amount,
        });

        Ok(())
    }

    pub fn get_total_weight(ctx: Context<GetTotalWeight>) -> Result<u32> {
        Ok(ctx.accounts.signer_list.total_weight)
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
pub struct CreateSignerList<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + SignerList::INIT_SPACE,
        seeds = [b"signer_list", owner.key().as_ref()],
        bump
    )]
    pub signer_list: Account<'info, SignerList>,
    
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddSigner<'info> {
    #[account(
        mut,
        seeds = [b"signer_list", owner.key().as_ref()],
        bump = signer_list.bump,
        has_one = owner
    )]
    pub signer_list: Account<'info, SignerList>,
    
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct RemoveSigner<'info> {
    #[account(
        mut,
        seeds = [b"signer_list", owner.key().as_ref()],
        bump = signer_list.bump,
        has_one = owner
    )]
    pub signer_list: Account<'info, SignerList>,
    
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + Proposal::INIT_SPACE,
        seeds = [b"proposal", owner.key().as_ref(), &owner.key().to_bytes()[..8]],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApproveProposal<'info> {
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    
    pub approver: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteProposal<'info> {
    #[account(
        mut,
        has_one = owner
    )]
    pub proposal: Account<'info, Proposal>,
    
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetTotalWeight<'info> {
    pub signer_list: Account<'info, SignerList>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub total_lists: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct SignerList {
    pub owner: Pubkey,
    pub total_weight: u32,
    pub signer_count: u32,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Proposal {
    pub owner: Pubkey,
    pub target: Pubkey,
    pub amount: u64,
    pub approvals_weight: u32,
    pub executed: bool,
    pub bump: u8,
}

#[event]
pub struct SignerListCreated {
    pub owner: Pubkey,
}

#[event]
pub struct SignerAdded {
    pub owner: Pubkey,
    pub signer: Pubkey,
    pub weight: u32,
}

#[event]
pub struct SignerRemoved {
    pub owner: Pubkey,
    pub weight: u32,
}

#[event]
pub struct ProposalCreated {
    pub owner: Pubkey,
    pub target: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ProposalApproved {
    pub owner: Pubkey,
    pub approver: Pubkey,
    pub weight: u32,
}

#[event]
pub struct ProposalExecuted {
    pub owner: Pubkey,
    pub target: Pubkey,
    pub amount: u64,
}

#[error_code]
pub enum SignerListError {
    #[msg("Proposal already executed")]
    AlreadyExecuted,
    #[msg("Quorum not met")]
    QuorumNotMet,
    #[msg("Invalid weight value")]
    InvalidWeight,
    #[msg("Insufficient weight")]
    InsufficientWeight,
    #[msg("No signers in list")]
    NoSigners,
}

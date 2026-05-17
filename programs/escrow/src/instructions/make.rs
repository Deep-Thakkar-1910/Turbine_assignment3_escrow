use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer_checked, TransferChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{error::EscrowError, Escrow, ESCROW_SEED};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
      mint::token_program = token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
      mint::token_program = token_program
    )]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
      mut,
      associated_token::mint = mint_a,
      associated_token::authority = maker,
      associated_token::token_program = token_program
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
      init,
      payer  = maker,
      seeds = [ESCROW_SEED,seed.to_le_bytes().as_ref(),maker.key().as_ref()],
      bump,
      space = Escrow::DISCRIMINATOR.len() + Escrow::INIT_SPACE
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
      init,
      payer = maker,
      associated_token::mint = mint_a,
      associated_token::authority = escrow,
      associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Make<'info> {
    pub fn validate_deposit_amount(&self, amount: u64, receive: u64) -> Result<()> {
        require_gt!(receive, 0, EscrowError::InvalidAmount);
        require_gt!(amount, 0, EscrowError::InvalidAmount);
        Ok(())
    }

    pub fn init_escrow(
        &mut self,
        seed: u64,
        receive: u64,
        expiry_timestamp: i64,
        bumps: &MakeBumps,
    ) -> Result<()> {
        require!(
            expiry_timestamp > Clock::get()?.unix_timestamp,
            EscrowError::InvalidExpiry
        );
        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            receive,
            bump: bumps.escrow,
            expiry_timestamp,
        });
        Ok(())
    }

    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        let transfer_accounts = TransferChecked {
            from: self.maker_ata_a.to_account_info(),
            to: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            authority: self.maker.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.key(), transfer_accounts);

        transfer_checked(cpi_ctx, amount, self.mint_a.decimals)?;

        Ok(())
    }
}

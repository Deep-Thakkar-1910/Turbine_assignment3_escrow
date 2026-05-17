use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{close_account, transfer_checked, CloseAccount, TransferChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{error::EscrowError, Escrow, ESCROW_SEED};

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(
      mint::token_program = token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
      mint::token_program = token_program
    )]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
      init_if_needed,
      payer = taker,
      associated_token::mint = mint_a,
      associated_token::authority = taker,
      associated_token::token_program = token_program
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
      mut,
      associated_token::mint = mint_b,
      associated_token::authority = taker,
      associated_token::token_program = token_program
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
      init_if_needed,
      payer = taker,
      associated_token::mint = mint_b,
      associated_token::authority = maker,
      associated_token::token_program = token_program
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
      mut,
      close = maker,
      seeds = [ESCROW_SEED,escrow.seed.to_le_bytes().as_ref(),escrow.maker.key().as_ref()],
      bump = escrow.bump,
      has_one = mint_a,
      has_one = mint_b,
      has_one = maker,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
      mut,
      associated_token::mint = mint_a,
      associated_token::authority = escrow,
      associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Take<'info> {
    pub fn validate_expiry(&self) -> Result<()> {
        require!(
            Clock::get()?.unix_timestamp < self.escrow.expiry_timestamp,
            EscrowError::EscrowExpired
        );
        Ok(())
    }

    // Transfering from taker to maker
    pub fn transfer(&mut self) -> Result<()> {
        let transfer_accounts = TransferChecked {
            from: self.taker_ata_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
            mint: self.mint_b.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.key(), transfer_accounts);

        transfer_checked(cpi_ctx, self.escrow.receive, self.mint_b.decimals)?;
        Ok(())
    }

    // Transfering from vault to taker and closing vault
    pub fn transfer_from_vault_and_close(&mut self) -> Result<()> {
        let transfer_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            to: self.taker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
            mint: self.mint_a.to_account_info(),
        };

        let escrow_seed_val = self.escrow.seed.to_le_bytes();
        let maker_key = self.maker.key();
        let seeds = &[
            ESCROW_SEED,
            escrow_seed_val.as_ref(),
            maker_key.as_ref(),
            &[self.escrow.bump],
        ];

        let signer_seeds = &[&seeds[..]];

        let cpi_ctx =
            CpiContext::new_with_signer(self.token_program.key(), transfer_accounts, signer_seeds);

        transfer_checked(cpi_ctx, self.vault.amount, self.mint_a.decimals)?;

        let close_ix_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let close_cpi_ctx =
            CpiContext::new_with_signer(self.token_program.key(), close_ix_accounts, signer_seeds);

        close_account(close_cpi_ctx)?;

        Ok(())
    }
}

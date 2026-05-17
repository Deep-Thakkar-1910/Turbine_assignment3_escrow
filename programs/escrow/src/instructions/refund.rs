use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{close_account, transfer_checked, CloseAccount, TransferChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{Escrow, ESCROW_SEED};

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
      mut,
      associated_token::mint = mint_a,
      associated_token::authority = maker,
      associated_token::token_program = token_program
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
      mint::token_program = token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
      mut,
      close = maker,
      seeds = [ESCROW_SEED,escrow.seed.to_le_bytes().as_ref(),escrow.maker.key().as_ref()],
      bump = escrow.bump,
      has_one = mint_a,
      has_one = maker
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
      mut,
      associated_token::mint = mint_a,
      associated_token::authority = escrow,
      associated_token::token_program = token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Refund<'info> {
    pub fn refund_and_close_vault(&mut self) -> Result<()> {
        let transfer_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            to: self.maker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
            mint: self.mint_a.to_account_info(),
        };

        let seed_val = self.escrow.seed.to_le_bytes();
        let maker_val = self.maker.key();
        let seeds = &[
            ESCROW_SEED,
            seed_val.as_ref(),
            maker_val.as_ref(),
            &[self.escrow.bump],
        ];

        let signer_seeds = &[&seeds[..]];

        let transfer_cpi_ctx =
            CpiContext::new_with_signer(self.token_program.key(), transfer_accounts, signer_seeds);

        transfer_checked(transfer_cpi_ctx, self.vault.amount, self.mint_a.decimals)?;

        let close_vault_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            authority: self.escrow.to_account_info(),
            destination: self.maker.to_account_info(),
        };

        let close_cpi_ctx = CpiContext::new_with_signer(
            self.token_program.key(),
            close_vault_accounts,
            signer_seeds,
        );

        close_account(close_cpi_ctx)?;
        Ok(())
    }
}

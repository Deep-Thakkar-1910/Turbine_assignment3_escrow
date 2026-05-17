pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("58AGNg2soLw1k8eBGMA3MLubjCaDqedyawUaCWZg2EPA");

#[program]
pub mod escrow {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        receive: u64,
        amount: u64,
        expiry_timestamp: i64,
    ) -> Result<()> {
        ctx.accounts.validate_deposit_amount(amount, receive)?;
        ctx.accounts
            .init_escrow(seed, receive, expiry_timestamp, &ctx.bumps)?;
        ctx.accounts.deposit(amount)
    }
    #[instruction(discriminator = 1)]
    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.validate_expiry()?; // Ensure take instruction cannot be performed on an expired escrow
        ctx.accounts.transfer()?;
        ctx.accounts.transfer_from_vault_and_close()
    }
    #[instruction(discriminator = 2)]
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund_and_close_vault()
    }
}

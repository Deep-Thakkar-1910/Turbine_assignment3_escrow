use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Can not withdraw funds after escrow expired")]
    EscrowExpired,
    #[msg("Invalid Amount")]
    InvalidAmount,
    #[msg("Invalid Expiry")]
    InvalidExpiry,
}

use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub fee: u16,
    pub treasury: Pubkey,
    pub authority: Option<Pubkey>,
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,
    pub seed: u64,
    pub locked: bool,
    pub config_bump: u8,
    pub lp_bump: u8,
}

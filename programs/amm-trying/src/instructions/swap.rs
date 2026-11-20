use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{error::AmmError, state::Config};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Box<Account<'info, Mint>>,
    pub mint_y: Box<Account<'info, Mint>>,
    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump
    )]
    pub config: Account<'info, Config>,
    #[account(
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub mint_lp: Box<Account<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Box<Account<'info, TokenAccount>>,
    
    /// CHECK: Validated against the treasury stored in Config.
    #[account(
        address = config.treasury
    )]
    pub treasury: UncheckedAccount<'info>,
    
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_x,
        associated_token::authority = treasury,
    )]
    pub treasury_x: Box<Account<'info, TokenAccount>>,
    
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_y,
        associated_token::authority = treasury,
    )]
    pub treasury_y: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Swap<'info> {
    
    pub fn swap(
        &mut self,
        is_x: bool,
        amount: u64,
        min: u64,
    ) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);
    
        // Calculate protocol fee in basis points.
        let fee = amount
            .checked_mul(self.config.fee as u64)
            .ok_or(AmmError::Overflow)?
            .checked_div(crate::constants::FEE_DENOMINATOR)
            .ok_or(AmmError::Overflow)?;
    
        // Amount that actually enters the AMM.
        let net_amount = amount
            .checked_sub(fee)
            .ok_or(AmmError::Underflow)?;
    
        require!(net_amount > 0, AmmError::InvalidAmount);
    
        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            0,
            Some(6),
        )
        .map_err(|_| AmmError::CurveError)?;
    
        let pair = if is_x {
            LiquidityPair::X
        } else {
            LiquidityPair::Y
        };
    
        let swap_result = curve
            .swap(pair, net_amount, min)
            .map_err(|_| AmmError::SlippageExceeded)?;
    
        self.collect_fee(is_x, fee)?;
        self.deposit_tokens(is_x, swap_result.deposit)?;
        self.withdraw_tokens(is_x, swap_result.withdraw)
    }

    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = if is_x {
            (
                self.user_x.to_account_info(),
                self.vault_x.to_account_info(),
            )
        } else {
            (
                self.user_y.to_account_info(),
                self.vault_y.to_account_info(),
            )
        };
    
        transfer(
            CpiContext::new(
                self.token_program.key(),
                Transfer {
                    from,
                    to,
                    authority: self.user.to_account_info(),
                },
            ),
            amount,
        )
    }

    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = if is_x {
            (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            )
        } else {
            (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            )
        };
    
        transfer(
            CpiContext::new_with_signer(
                self.token_program.key(),
                Transfer {
                    from,
                    to,
                    authority: self.config.to_account_info(),
                },
                &[&[
                    b"config",
                    &self.config.seed.to_le_bytes(),
                    &[self.config.config_bump],
                ]],
            ),
            amount,
        )
    }
    
    pub fn collect_fee(&self, is_x: bool, amount: u64) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }
    
        let (from, to) = if is_x {
            (
                self.user_x.to_account_info(),
                self.treasury_x.to_account_info(),
            )
        } else {
            (
                self.user_y.to_account_info(),
                self.treasury_y.to_account_info(),
            )
        };
    
        transfer(
            CpiContext::new(
                self.token_program.key(),
                Transfer {
                    from,
                    to,
                    authority: self.user.to_account_info(),
                },
            ),
            amount,
        )
    }


}

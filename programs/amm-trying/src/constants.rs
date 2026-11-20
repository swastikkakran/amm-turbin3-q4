use anchor_lang::prelude::*;

#[constant]
pub const SEED: &str = "anchor";

pub const FEE_DENOMINATOR: u64 = 10_000;
pub const MAX_FEE_BPS: u16 = 1_000;

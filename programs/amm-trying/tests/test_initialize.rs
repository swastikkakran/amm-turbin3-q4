use anchor_lang::{prelude::*, solana_program::instruction::Instruction, InstructionData, ToAccountMetas};
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};
use litesvm::LiteSVM;
use litesvm_token::{
    get_spl_account,
    spl_token::state::Account as SplTokenAccount,
    CreateAssociatedTokenAccount, CreateMint, MintTo,
};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

use amm_trying::{accounts, instruction, ID};

const LAMPORTS: u64 = 10_000_000_000;
const TOKEN_AMOUNT: u64 = 10_000_000_000;
const INITIAL_LIQUIDITY: u64 = 1_000_000_000;
const SWAP_AMOUNT: u64 = 100_000_000;
const FEE_BPS: u16 = 100; // 1%
const SEED: u64 = 42;

struct TestContext {
    svm: LiteSVM,
    payer: Keypair,
    treasury: Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    config: Pubkey,
    mint_lp: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
    user_x: Pubkey,
    user_y: Pubkey,
    user_lp: Pubkey,
    treasury_x: Pubkey,
    treasury_y: Pubkey,
}

impl TestContext {
    fn new(fee: u16) -> Self {
        let mut ctx = Self::new_uninitialized();
        ctx.send(ctx.initialize_ix(fee)).expect("initialize pool");
        ctx
    }

    fn new_uninitialized() -> Self {
        let mut svm = LiteSVM::new();
        let program_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy/amm_trying.so");
        svm.add_program_from_file(ID, program_path)
            .unwrap_or_else(|err| {
                panic!("Failed to load AMM program from {program_path}: {err:?}");
            });

        let payer = Keypair::new();
        let treasury = Keypair::new();
        svm.airdrop(&payer.pubkey(), LAMPORTS).unwrap();
        svm.airdrop(&treasury.pubkey(), LAMPORTS).unwrap();

        let mint_x = CreateMint::new(&mut svm, &payer)
            .authority(&payer.pubkey())
            .decimals(6)
            .send()
            .unwrap();
        let mint_y = CreateMint::new(&mut svm, &payer)
            .authority(&payer.pubkey())
            .decimals(6)
            .send()
            .unwrap();

        let user_x = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_x)
            .owner(&payer.pubkey())
            .send()
            .unwrap();
        let user_y = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_y)
            .owner(&payer.pubkey())
            .send()
            .unwrap();

        MintTo::new(&mut svm, &payer, &mint_x, &user_x, TOKEN_AMOUNT)
            .owner(&payer)
            .send()
            .unwrap();
        MintTo::new(&mut svm, &payer, &mint_y, &user_y, TOKEN_AMOUNT)
            .owner(&payer)
            .send()
            .unwrap();

        let (config, _) = Pubkey::find_program_address(
            &[b"config", &SEED.to_le_bytes()],
            &ID,
        );
        let (mint_lp, _) = Pubkey::find_program_address(&[b"lp", config.as_ref()], &ID);
        let vault_x = ata(&config, &mint_x);
        let vault_y = ata(&config, &mint_y);
        let user_lp = ata(&payer.pubkey(), &mint_lp);
        let treasury_x = ata(&treasury.pubkey(), &mint_x);
        let treasury_y = ata(&treasury.pubkey(), &mint_y);

        Self {
            svm,
            payer,
            treasury,
            mint_x,
            mint_y,
            config,
            mint_lp,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            treasury_x,
            treasury_y,
        }
    }

    fn initialize_ix(&self, fee: u16) -> Instruction {
        Instruction {
            program_id: ID,
            accounts: accounts::Initialize {
                initializer: self.payer.pubkey(),
                treasury: self.treasury.pubkey(),
                mint_x: self.mint_x,
                mint_y: self.mint_y,
                mint_lp: self.mint_lp,
                vault_x: self.vault_x,
                vault_y: self.vault_y,
                config: self.config,
                system_program: system_program::ID,
                token_program: token::ID,
                associated_token_program: associated_token::ID,
            }
            .to_account_metas(None),
            data: instruction::Initialize {
                seed: SEED,
                fee,
                authority: None,
            }
            .data(),
        }
    }

    fn send(&mut self, ix: Instruction) -> std::result::Result<(), String> {
        use solana_signer::Signer;
        use solana_transaction::Transaction;
    
        let mut tx = Transaction::new_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
        );
    
        tx.sign(
            &[&self.payer],
            self.svm.latest_blockhash(),
        );
    
        self.svm
            .send_transaction(tx)
            .map(|_| ())
            .map_err(|err| format!("{err:?}"))
    }

    fn deposit_ix(&self, lp_amount: u64, max_x: u64, max_y: u64) -> Instruction {
        Instruction {
            program_id: ID,
            accounts: accounts::Deposit {
                user: self.payer.pubkey(),
                mint_x: self.mint_x,
                mint_y: self.mint_y,
                config: self.config,
                mint_lp: self.mint_lp,
                vault_x: self.vault_x,
                vault_y: self.vault_y,
                user_x: self.user_x,
                user_y: self.user_y,
                user_lp: self.user_lp,
                token_program: token::ID,
                system_program: system_program::ID,
                associated_token_program: associated_token::ID,
            }
            .to_account_metas(None),
            data: instruction::Deposit {
                amount: lp_amount,
                max_x,
                max_y,
            }
            .data(),
        }
    }

    fn swap_ix(&self, is_x: bool, amount_in: u64, min_out: u64) -> Instruction {
        Instruction {
            program_id: ID,
            accounts: accounts::Swap {
                user: self.payer.pubkey(),
                mint_x: self.mint_x,
                mint_y: self.mint_y,
                config: self.config,
                mint_lp: self.mint_lp,
                vault_x: self.vault_x,
                vault_y: self.vault_y,
                user_x: self.user_x,
                user_y: self.user_y,
                treasury: self.treasury.pubkey(),
                treasury_x: self.treasury_x,
                treasury_y: self.treasury_y,
                token_program: token::ID,
                system_program: system_program::ID,
                associated_token_program: associated_token::ID,
            }
            .to_account_metas(None),
            data: instruction::Swap {
                is_x,
                amount_in,
                min_amount_out: min_out,
            }
            .data(),
        }
    }

    fn withdraw_ix(&self, lp_amount: u64, min_x: u64, min_y: u64) -> Instruction {
        Instruction {
            program_id: ID,
            accounts: accounts::Withdraw {
                user: self.payer.pubkey(),
                mint_x: self.mint_x,
                mint_y: self.mint_y,
                config: self.config,
                mint_lp: self.mint_lp,
                vault_x: self.vault_x,
                vault_y: self.vault_y,
                user_x: self.user_x,
                user_y: self.user_y,
                user_lp: self.user_lp,
                token_program: token::ID,
                system_program: system_program::ID,
                associated_token_program: associated_token::ID,
            }
            .to_account_metas(None),
            data: instruction::Withdraw {
                amount: lp_amount,
                min_x,
                min_y,
            }
            .data(),
        }
    }

    fn token_balance(&self, address: &Pubkey) -> u64 {
        let account: SplTokenAccount = get_spl_account(&self.svm, address).unwrap();
        account.amount
    }

    fn initialize_lp(&mut self) {
        self.send(self.deposit_ix(
            INITIAL_LIQUIDITY,
            INITIAL_LIQUIDITY,
            INITIAL_LIQUIDITY,
        ))
        .expect("initial liquidity deposit");
    }
}

fn ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token::ID.as_ref(), mint.as_ref()],
        &associated_token::ID,
    )
    .0
}

#[test]
fn initialize_deposit_swap_both_directions_and_withdraw() {
    let mut ctx = TestContext::new(FEE_BPS);

    // initialize() created the config, LP mint and both pool vaults.
    assert!(ctx.svm.get_account(&ctx.config).is_some());
    assert!(ctx.svm.get_account(&ctx.vault_x).is_some());
    assert!(ctx.svm.get_account(&ctx.vault_y).is_some());

    // deposit(): initial liquidity.
    ctx.initialize_lp();
    assert_eq!(ctx.token_balance(&ctx.vault_x), INITIAL_LIQUIDITY);
    assert_eq!(ctx.token_balance(&ctx.vault_y), INITIAL_LIQUIDITY);
    assert_eq!(ctx.token_balance(&ctx.user_lp), INITIAL_LIQUIDITY);

    // deposit(): proportional follow-up liquidity.
    let second_lp = 100_000_000;
    ctx.send(ctx.deposit_ix(second_lp, second_lp, second_lp))
        .expect("proportional deposit");
    assert_eq!(ctx.token_balance(&ctx.user_lp), INITIAL_LIQUIDITY + second_lp);

    // swap X -> Y: 1% fee should be transferred to treasury X.
    let treasury_x_before = 0;
    let vault_x_before = ctx.token_balance(&ctx.vault_x);
    ctx.send(ctx.swap_ix(true, SWAP_AMOUNT, 1))
        .expect("swap X to Y");
    let expected_fee = SWAP_AMOUNT * FEE_BPS as u64 / 10_000;
    assert_eq!(ctx.token_balance(&ctx.treasury_x), treasury_x_before + expected_fee);
    assert_eq!(
        ctx.token_balance(&ctx.vault_x),
        vault_x_before + SWAP_AMOUNT - expected_fee
    );

    // swap Y -> X: fee must accrue to treasury Y, not treasury X.
    let treasury_y_before = ctx.token_balance(&ctx.treasury_y);
    let vault_y_before = ctx.token_balance(&ctx.vault_y);
    ctx.send(ctx.swap_ix(false, SWAP_AMOUNT, 1))
        .expect("swap Y to X");
    assert_eq!(
        ctx.token_balance(&ctx.treasury_y),
        treasury_y_before + expected_fee
    );
    assert_eq!(
        ctx.token_balance(&ctx.vault_y),
        vault_y_before + SWAP_AMOUNT - expected_fee
    );

    // withdraw(): burn LP and return both pool assets.
    let user_x_before = ctx.token_balance(&ctx.user_x);
    let user_y_before = ctx.token_balance(&ctx.user_y);
    let lp_before = ctx.token_balance(&ctx.user_lp);
    ctx.send(ctx.withdraw_ix(100_000_000, 1, 1))
        .expect("withdraw liquidity");
    assert_eq!(ctx.token_balance(&ctx.user_lp), lp_before - 100_000_000);
    assert!(ctx.token_balance(&ctx.user_x) > user_x_before);
    assert!(ctx.token_balance(&ctx.user_y) > user_y_before);
}

#[test]
fn initialize_rejects_fee_above_maximum() {
    // MAX_FEE_BPS is 1000 (10%). 1001 must be rejected.
    let mut ctx = TestContext::new_uninitialized();
    let err = ctx.send(ctx.initialize_ix(1_001));
    assert!(err.is_err(), "initialize must reject fees above MAX_FEE_BPS");
}

#[test]
fn deposit_rejects_zero_lp_amount() {
    let mut ctx = TestContext::new(FEE_BPS);
    let err = ctx.send(ctx.deposit_ix(0, 0, 0));
    assert!(err.is_err(), "zero LP deposit must fail");
}

#[test]
fn swap_rejects_zero_input() {
    let mut ctx = TestContext::new(FEE_BPS);
    ctx.initialize_lp();
    let err = ctx.send(ctx.swap_ix(true, 0, 0));
    assert!(err.is_err(), "zero-input swap must fail");
}

#[test]
fn swap_rejects_unreachable_minimum_output_without_changing_balances() {
    let mut ctx = TestContext::new(FEE_BPS);
    ctx.initialize_lp();
    let user_x_before = ctx.token_balance(&ctx.user_x);
    let treasury_x_before = if ctx.svm.get_account(&ctx.treasury_x).is_some() {
        ctx.token_balance(&ctx.treasury_x)
    } else {
        0
    };

    let err = ctx.send(ctx.swap_ix(true, SWAP_AMOUNT, u64::MAX));
    assert!(err.is_err(), "impossible minimum output must fail");
    assert_eq!(ctx.token_balance(&ctx.user_x), user_x_before);
    assert_eq!(
        if ctx.svm.get_account(&ctx.treasury_x).is_some() {
            ctx.token_balance(&ctx.treasury_x)
        } else {
            0
        },
        treasury_x_before,
        "failed swap must roll back treasury transfer"
    );
}

#[test]
fn withdraw_rejects_zero_lp_amount() {
    let mut ctx = TestContext::new(FEE_BPS);
    ctx.initialize_lp();
    let err = ctx.send(ctx.withdraw_ix(0, 0, 0));
    assert!(err.is_err(), "zero LP withdrawal must fail");
}

#[test]
fn withdraw_rejects_unreachable_minimums() {
    let mut ctx = TestContext::new(FEE_BPS);
    ctx.initialize_lp();
    let lp_before = ctx.token_balance(&ctx.user_lp);
    let err = ctx.send(ctx.withdraw_ix(100_000_000, u64::MAX, u64::MAX));
    assert!(err.is_err(), "impossible withdrawal minimums must fail");
    assert_eq!(ctx.token_balance(&ctx.user_lp), lp_before);
}

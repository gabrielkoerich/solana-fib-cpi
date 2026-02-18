use solana_program_test::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
    transaction::Transaction,
};

const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);

const SEED: &[u8] = b"fib";

fn program_test() -> (ProgramTest, Pubkey) {
    let id = Pubkey::new_unique();
    let pt = ProgramTest::new(
        "solana_fib_cpi",
        id,
        processor!(solana_fib_cpi::process_instruction),
    );
    (pt, id)
}

fn init_ix(program_id: Pubkey, pda: Pubkey, payer: Pubkey, n: u64) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(pda, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: n.to_le_bytes().to_vec(),
    }
}

fn read_state(data: &[u8]) -> (u64, u64, u64, u8) {
    let a = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let b = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let n = u64::from_le_bytes(data[16..24].try_into().unwrap());
    let bump = data[24];
    (a, b, n, bump)
}

#[tokio::test]
async fn init_n_zero() {
    let (pt, pid) = program_test();
    let (banks, payer, bh) = pt.start().await;

    let (pda, bump) = Pubkey::find_program_address(&[SEED, payer.pubkey().as_ref()], &pid);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix(pid, pda, payer.pubkey(), 0)],
        Some(&payer.pubkey()),
        &[&payer],
        bh,
    );
    banks.process_transaction(tx).await.unwrap();

    let acct = banks.get_account(pda).await.unwrap().unwrap();
    let (a, b, n, stored_bump) = read_state(&acct.data);
    assert_eq!(a, 0);
    assert_eq!(b, 1);
    assert_eq!(n, 0);
    assert_eq!(stored_bump, bump);
    assert_eq!(acct.owner, pid);
}

#[tokio::test]
async fn fibonacci_n1() {
    let (pt, pid) = program_test();
    let (banks, payer, bh) = pt.start().await;
    let (pda, _) = Pubkey::find_program_address(&[SEED, payer.pubkey().as_ref()], &pid);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix(pid, pda, payer.pubkey(), 1)],
        Some(&payer.pubkey()),
        &[&payer],
        bh,
    );
    banks.process_transaction(tx).await.unwrap();

    // init: a=0, b=1, n=1
    // step: a=1, b=0+1=1, n=0
    let acct = banks.get_account(pda).await.unwrap().unwrap();
    let (a, b, n, _) = read_state(&acct.data);
    assert_eq!(a, 1);
    assert_eq!(b, 1);
    assert_eq!(n, 0);
}

#[tokio::test]
async fn fibonacci_n3() {
    let (pt, pid) = program_test();
    let (banks, payer, bh) = pt.start().await;
    let (pda, _) = Pubkey::find_program_address(&[SEED, payer.pubkey().as_ref()], &pid);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix(pid, pda, payer.pubkey(), 3)],
        Some(&payer.pubkey()),
        &[&payer],
        bh,
    );
    banks.process_transaction(tx).await.unwrap();

    // init:   a=0, b=1, n=3
    // step 1: a=1, b=1, n=2
    // step 2: a=1, b=2, n=1
    // step 3: a=2, b=3, n=0
    let acct = banks.get_account(pda).await.unwrap().unwrap();
    let (a, b, n, _) = read_state(&acct.data);
    assert_eq!(a, 2);
    assert_eq!(b, 3);
    assert_eq!(n, 0);
}

#[tokio::test]
async fn fibonacci_n4_max_depth() {
    // CPI stack height limit = 5 (invoke depth 1 + 4 CPI levels).
    // init(h=1) -> step(h=2) -> step(h=3) -> step(h=4) -> step(h=5) = OK
    let (pt, pid) = program_test();
    let (banks, payer, bh) = pt.start().await;
    let (pda, _) = Pubkey::find_program_address(&[SEED, payer.pubkey().as_ref()], &pid);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix(pid, pda, payer.pubkey(), 4)],
        Some(&payer.pubkey()),
        &[&payer],
        bh,
    );
    banks.process_transaction(tx).await.unwrap();

    // init:   a=0, b=1, n=4
    // step 1: a=1, b=1, n=3
    // step 2: a=1, b=2, n=2
    // step 3: a=2, b=3, n=1
    // step 4: a=3, b=5, n=0
    let acct = banks.get_account(pda).await.unwrap().unwrap();
    let (a, b, n, _) = read_state(&acct.data);
    assert_eq!(a, 3);
    assert_eq!(b, 5);
    assert_eq!(n, 0);
}

#[tokio::test]
async fn fibonacci_n5_exceeds_depth() {
    // n=5 needs 5 self-CPI calls reaching stack height 6 > limit of 5.
    let (pt, pid) = program_test();
    let (banks, payer, bh) = pt.start().await;
    let (pda, _) = Pubkey::find_program_address(&[SEED, payer.pubkey().as_ref()], &pid);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix(pid, pda, payer.pubkey(), 5)],
        Some(&payer.pubkey()),
        &[&payer],
        bh,
    );
    let result = banks.process_transaction(tx).await;
    assert!(result.is_err(), "n=5 should exceed CPI depth limit");
}

#[tokio::test]
async fn wrong_pda_fails() {
    let (pt, pid) = program_test();
    let (banks, payer, bh) = pt.start().await;

    let wrong_pda = Pubkey::new_unique();
    let tx = Transaction::new_signed_with_payer(
        &[init_ix(pid, wrong_pda, payer.pubkey(), 0)],
        Some(&payer.pubkey()),
        &[&payer],
        bh,
    );
    let result = banks.process_transaction(tx).await;
    assert!(result.is_err(), "wrong PDA should fail");
}

#![cfg(test)]
mod tests {

    use {
        anchor_lang::{
            prelude::*,
            solana_program::{
                instruction::Instruction, program_pack::Pack,
                system_program::ID as SYSTEM_PROGRAM_ID,
            },
            InstructionData, ToAccountMetas,
        },
        anchor_spl::{
            associated_token::{
                get_associated_token_address_with_program_id, ID as ASSOCIATED_TOKEN_PROGRAM_ID,
            },
            token::spl_token,
        },
        litesvm::LiteSVM,
        litesvm_token::{
            spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
        },
        solana_keypair::Keypair,
        solana_pubkey::Pubkey,
        solana_signer::Signer,
        solana_transaction::Transaction,
    };

    fn setup() -> (LiteSVM, Keypair) {
        let program_id = escrow::id();

        let payer = Keypair::new();

        let mut svm = LiteSVM::new();

        let bytes: &[u8] = include_bytes!("../../../target/deploy/escrow.so");

        svm.add_program(program_id, bytes).unwrap();

        svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

        (svm, payer)
    }

    #[test]
    fn test_make_and_take() {
        let (mut program, payer) = setup();

        let maker = Keypair::new();
        let taker = Keypair::new();

        program.airdrop(&maker.pubkey(), 1_000_000_000).unwrap();

        program.airdrop(&taker.pubkey(), 1_000_000_000).unwrap();

        let mint_a = CreateMint::new(&mut program, &payer).send().unwrap();

        let mint_b = CreateMint::new(&mut program, &payer).send().unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1_000_000)
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 2_000_000)
            .send()
            .unwrap();

        let seed = 69u64;

        let (escrow, _) = Pubkey::find_program_address(
            &[b"escrow", &seed.to_le_bytes(), maker.pubkey().as_ref()],
            &escrow::id(),
        );

        let vault =
            get_associated_token_address_with_program_id(&escrow, &mint_a, &TOKEN_PROGRAM_ID);

        // Make
        let expiry_timestamp = Clock::default().unix_timestamp + 100;

        let make_ix = Instruction {
            program_id: escrow::id(),

            accounts: escrow::accounts::Make {
                maker: maker.pubkey(),

                mint_a,
                mint_b,

                maker_ata_a,

                escrow,
                vault,

                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),

            data: escrow::instruction::Make {
                seed,
                amount: 1_000_000,
                receive: 2_000_000,
                expiry_timestamp,
            }
            .data(),
        };

        let make_tx = Transaction::new_signed_with_payer(
            &[make_ix],
            Some(&payer.pubkey()),
            &[&payer, &maker],
            program.latest_blockhash(),
        );

        program.send_transaction(make_tx).unwrap();

        // Take
        let take_ix = Instruction {
            program_id: escrow::id(),

            accounts: escrow::accounts::Take {
                taker: taker.pubkey(),
                maker: maker.pubkey(),

                mint_a,
                mint_b,

                taker_ata_a,
                taker_ata_b,

                maker_ata_b,

                escrow,
                vault,

                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),

            data: escrow::instruction::Take {}.data(),
        };

        let take_tx = Transaction::new_signed_with_payer(
            &[take_ix],
            Some(&payer.pubkey()),
            &[&payer, &taker],
            program.latest_blockhash(),
        );

        program.send_transaction(take_tx).unwrap();

        let taker_token_a_account = program.get_account(&taker_ata_a).unwrap();

        let maker_token_b_account = program.get_account(&maker_ata_b).unwrap();

        let taker_token_a = spl_token::state::Account::unpack(&taker_token_a_account.data).unwrap();

        let maker_token_b = spl_token::state::Account::unpack(&maker_token_b_account.data).unwrap();

        assert_eq!(taker_token_a.amount, 1_000_000);

        assert_eq!(maker_token_b.amount, 2_000_000);
    }

    #[test]
    fn test_make_and_refund() {
        let (mut program, payer) = setup();

        let maker = Keypair::new();
        let taker = Keypair::new();

        program.airdrop(&maker.pubkey(), 1_000_000_000).unwrap();

        program.airdrop(&taker.pubkey(), 1_000_000_000).unwrap();

        let mint_a = CreateMint::new(&mut program, &payer).send().unwrap();

        let mint_b = CreateMint::new(&mut program, &payer).send().unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1_000_000)
            .send()
            .unwrap();

        let seed = 69u64;

        let (escrow, _) = Pubkey::find_program_address(
            &[b"escrow", &seed.to_le_bytes(), maker.pubkey().as_ref()],
            &escrow::id(),
        );

        let vault =
            get_associated_token_address_with_program_id(&escrow, &mint_a, &TOKEN_PROGRAM_ID);

        // Make
        let expiry_timestamp = Clock::default().unix_timestamp + 100;

        let make_ix = Instruction {
            program_id: escrow::id(),

            accounts: escrow::accounts::Make {
                maker: maker.pubkey(),

                mint_a,
                mint_b,

                maker_ata_a,

                escrow,
                vault,

                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),

            data: escrow::instruction::Make {
                seed,
                amount: 1_000_000,
                receive: 2_000_000,
                expiry_timestamp,
            }
            .data(),
        };

        let make_tx = Transaction::new_signed_with_payer(
            &[make_ix],
            Some(&payer.pubkey()),
            &[&payer, &maker],
            program.latest_blockhash(),
        );

        program.send_transaction(make_tx).unwrap();

        let vault_acc = program.get_account(&vault).unwrap();
        let vault_state = spl_token::state::Account::unpack(&vault_acc.data).unwrap();
        assert_eq!(vault_state.amount, 1_000_000);

        // Refund
        let refund_ix = Instruction {
            program_id: escrow::id(),

            accounts: escrow::accounts::Refund {
                maker: maker.pubkey(),
                maker_ata_a,
                mint_a,
                escrow,
                vault,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),

            data: escrow::instruction::Refund {}.data(),
        };

        let refund_tx = Transaction::new_signed_with_payer(
            &[refund_ix],
            Some(&payer.pubkey()),
            &[&payer, &maker],
            program.latest_blockhash(),
        );

        program.send_transaction(refund_tx).unwrap();

        let maker_ata_a_acc = program.get_account(&maker_ata_a).unwrap();
        let maker_ata_a_state = spl_token::state::Account::unpack(&maker_ata_a_acc.data).unwrap();
        assert_eq!(maker_ata_a_state.amount, 1_000_000);

        assert!(program.get_account(&vault).is_none());
    }

    #[test]
    #[should_panic(expected = "EscrowExpired")]
    fn test_take_after_expiry() {
        let (mut program, payer) = setup();

        let maker = Keypair::new();
        let taker = Keypair::new();

        program.airdrop(&maker.pubkey(), 1_000_000_000).unwrap();
        program.airdrop(&taker.pubkey(), 1_000_000_000).unwrap();

        let mint_a = CreateMint::new(&mut program, &payer).send().unwrap();
        let mint_b = CreateMint::new(&mut program, &payer).send().unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1_000_000)
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 2_000_000)
            .send()
            .unwrap();

        let seed = 999u64;

        let (escrow, _) = Pubkey::find_program_address(
            &[b"escrow", &seed.to_le_bytes(), maker.pubkey().as_ref()],
            &escrow::id(),
        );

        let vault =
            get_associated_token_address_with_program_id(&escrow, &mint_a, &TOKEN_PROGRAM_ID);

        let expiry_timestamp = Clock::default().unix_timestamp + 10;

        // Make
        let make_ix = Instruction {
            program_id: escrow::id(),

            accounts: escrow::accounts::Make {
                maker: maker.pubkey(),

                mint_a,
                mint_b,

                maker_ata_a,

                escrow,
                vault,

                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),

            data: escrow::instruction::Make {
                seed,
                amount: 1_000_000,
                receive: 2_000_000,
                expiry_timestamp,
            }
            .data(),
        };

        let make_tx = Transaction::new_signed_with_payer(
            &[make_ix],
            Some(&payer.pubkey()),
            &[&payer, &maker],
            program.latest_blockhash(),
        );

        program.send_transaction(make_tx).unwrap();

        // Warp clock past expiry
        let mut clock = Clock::default();
        clock.unix_timestamp = expiry_timestamp + 1;
        program.set_sysvar(&clock);

        // Take with expired timestamp
        let take_ix = Instruction {
            program_id: escrow::id(),

            accounts: escrow::accounts::Take {
                taker: taker.pubkey(),
                maker: maker.pubkey(),

                mint_a,
                mint_b,

                taker_ata_a,
                taker_ata_b,

                maker_ata_b,

                escrow,
                vault,

                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),

            data: escrow::instruction::Take {}.data(),
        };

        let take_tx = Transaction::new_signed_with_payer(
            &[take_ix],
            Some(&payer.pubkey()),
            &[&payer, &taker],
            program.latest_blockhash(),
        );

        program.send_transaction(take_tx).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidAmount")]
    fn test_make_with_invalid_amount() {
        let (mut program, payer) = setup();

        let maker = Keypair::new();
        let taker = Keypair::new();

        program.airdrop(&maker.pubkey(), 1_000_000_000).unwrap();
        program.airdrop(&taker.pubkey(), 1_000_000_000).unwrap();

        let mint_a = CreateMint::new(&mut program, &payer).send().unwrap();
        let mint_b = CreateMint::new(&mut program, &payer).send().unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1_000_000)
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 2_000_000)
            .send()
            .unwrap();

        let seed = 999u64;

        let (escrow, _) = Pubkey::find_program_address(
            &[b"escrow", &seed.to_le_bytes(), maker.pubkey().as_ref()],
            &escrow::id(),
        );

        let vault =
            get_associated_token_address_with_program_id(&escrow, &mint_a, &TOKEN_PROGRAM_ID);

        let expiry_timestamp = Clock::default().unix_timestamp - 1;

        // Make Should fail with invalid amount
        let make_ix = Instruction {
            program_id: escrow::id(),

            accounts: escrow::accounts::Make {
                maker: maker.pubkey(),

                mint_a,
                mint_b,

                maker_ata_a,

                escrow,
                vault,

                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),

            data: escrow::instruction::Make {
                seed,
                amount: 0,
                receive: 0,
                expiry_timestamp,
            }
            .data(),
        };

        let make_tx = Transaction::new_signed_with_payer(
            &[make_ix],
            Some(&payer.pubkey()),
            &[&payer, &maker],
            program.latest_blockhash(),
        );

        program.send_transaction(make_tx).unwrap();
    }
}

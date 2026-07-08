use pinocchio::{
    account_info::AccountInfo, instruction::{Seed, Signer}, msg, pubkey::log, sysvars::{rent::Rent, Sysvar}, ProgramResult
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

use crate::state::Escrow;

pub fn process_make_instruction_v2(
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {

    msg!("Processing Make instruction V2");

    // Don't destructure initially - we'll access accounts by index to avoid borrow issues
    if accounts.len() < 9 {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    }

    // Initial validation using temporary references
    {
        let maker_ata = &accounts[4];
        let maker = &accounts[0];
        let mint_a = &accounts[1];
        
        let maker_ata_state = pinocchio_token::state::TokenAccount::from_account_info(maker_ata)?;
        if maker_ata_state.owner() != maker.key() {
            return Err(pinocchio::program_error::ProgramError::IllegalOwner);
        }
        if maker_ata_state.mint() != mint_a.key() {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
    }

    let bump = data[0];
    
    // PDA validation
    {
        let maker = &accounts[0];
        let escrow_account = &accounts[3];
        let seed = [b"escrow".as_ref(), maker.key().as_slice(), &[bump]];
        let escrow_account_pda = derive_address(&seed, None, &crate::ID);
        log(&escrow_account_pda);
        log(&escrow_account.key());
        assert_eq!(escrow_account_pda, *escrow_account.key());
    }

    let amount_to_receive = unsafe{ *(data.as_ptr().add(1) as *const u64) };
    let amount_to_give = unsafe{ *(data.as_ptr().add(9) as *const u64) };

    // Create escrow account if needed
    {
        let maker = &accounts[0];
        let escrow_account = &accounts[3];
        let mint_a = &accounts[1];
        let mint_b = &accounts[2];
        
        let bump_bytes = [data[0].to_le()];
        let seed = [Seed::from(b"escrow"), Seed::from(maker.key()), Seed::from(&bump_bytes)];
        let seeds = Signer::from(&seed);

        if escrow_account.owner() != &crate::ID {
            CreateAccount {
                from: maker,
                to: escrow_account,
                lamports: Rent::get()?.minimum_balance(Escrow::LEN),
                space: Escrow::LEN as u64,
                owner: &crate::ID,
            }.invoke_signed(&[seeds.clone()])?;

            let escrow_state = Escrow::from_account_info(escrow_account)?;
        
            escrow_state.set_maker(maker.key());
            escrow_state.set_mint_a(mint_a.key());
            escrow_state.set_mint_b(mint_b.key());
            escrow_state.set_amount_to_receive(amount_to_receive);
            escrow_state.set_amount_to_give(amount_to_give);  
            escrow_state.bump = data[0];
        }
        else {
            return Err(pinocchio::program_error::ProgramError::IllegalOwner);
        }
    }

    // Create the escrow ATA in a separate scope to release borrows
    {
        let maker = &accounts[0];
        let escrow_ata = &accounts[5];
        let escrow_account = &accounts[3];
        let mint_a = &accounts[1];
        let token_program = &accounts[7];
        let system_program = &accounts[6];
        
        pinocchio_associated_token_account::instructions::Create {
            funding_account: maker,
            account: escrow_ata,
            wallet: escrow_account,
            mint: mint_a,
            token_program: token_program,
            system_program: system_program,
        }.invoke()?;
    }
    // All references from the Create instruction are now dropped

    // Transfer tokens in a new scope with fresh references
    {
        let maker = &accounts[0];
        let maker_ata = &accounts[4];
        let escrow_ata = &accounts[5];
        
        pinocchio_token::instructions::Transfer {
            from: maker_ata,
            to: escrow_ata,
            authority: maker,
            amount: amount_to_give,
        }.invoke()?;
    }

    Ok(())
}

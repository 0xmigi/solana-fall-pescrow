use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError, sysvars::{Sysvar, rent::Rent}
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

use crate::state::Escrow;

/// 1 (bump) + 8 (amount_to_receive) + 8 (amount_to_give)
const MAKE_DATA_LEN: usize = 17;

pub fn process_make_instruction(
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {

    let [
        maker,
        mint_a,
        mint_b,
        escrow_account,
        maker_ata,
        escrow_ata,
        system_program,
        token_program,
        _associated_token_program@ ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Scope the borrow so it is released before any CPI below borrows `maker_ata`.
    {
        let maker_ata_state = pinocchio_token::state::TokenAccount::from_account_view(&maker_ata)?;
        if maker_ata_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if maker_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Instruction data layout (after the discriminator byte):
    //   [0]      bump: u8
    //   [1..9]   amount_to_receive: u64 (little-endian)
    //   [9..17]  amount_to_give: u64 (little-endian)
    if data.len() < MAKE_DATA_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }
    let bump = data[0];
    let amount_to_receive = u64::from_le_bytes(data[1..9].try_into().unwrap());
    let amount_to_give = u64::from_le_bytes(data[9..17].try_into().unwrap());

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];
    let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    if escrow_account_pda != *escrow_account.address().as_array() {
        return Err(ProgramError::InvalidSeeds);
    }

    let bump_bytes = [bump];
    let seed = [Seed::from(b"escrow"), Seed::from(maker.address().as_array()), Seed::from(&bump_bytes)];
    let seeds = Signer::from(&seed);

    unsafe {
        if escrow_account.owner() != &crate::ID {
            CreateAccount {
                from: maker,
                to: escrow_account,
                lamports: Rent::get()?.try_minimum_balance(Escrow::LEN)?,
                space: Escrow::LEN as u64,
                owner: &crate::ID,
            }.invoke_signed(&[seeds.clone()])?;


            {
                let escrow_state = Escrow::from_account_info(escrow_account)?;
            
                escrow_state.set_maker(maker.address());
                escrow_state.set_mint_a(mint_a.address());
                escrow_state.set_mint_b(mint_b.address());
                escrow_state.set_amount_to_receive(amount_to_receive);
                escrow_state.set_amount_to_give(amount_to_give);  
                escrow_state.bump = bump;
            }
        }
        else {
            return Err(ProgramError::IllegalOwner);
        }
    }

    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: escrow_ata,
        wallet: escrow_account,
        mint: mint_a,
        token_program: token_program,
        system_program: system_program,
    }.invoke()?;

    pinocchio_token::instructions::Transfer {
        from: maker_ata,
        to: escrow_ata,
        authority: maker,
        amount: amount_to_give,
    }.invoke()?;

    Ok(())
}
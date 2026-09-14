# Pescrow: A Pinocchio Escrow Challenge

> **Solana Fall School** · Native Rust on Solana with [Pinocchio](https://github.com/anza-xyz/pinocchio)

This repository is a *deliberately unfinished* escrow program. It ships with the **Make** instruction fully working and tested, and leaves **Take** and **Cancel** for you to build.

If you have only ever written Solana programs with Anchor, this is your chance to see what the framework was doing for you: manual account validation, manual PDA derivation, zero-copy state, and raw CPIs, all with a fraction of the compute cost.

**Table of contents**

1. [What is an escrow?](#1-what-is-an-escrow)
2. [Getting started](#2-getting-started)
3. [Code walkthrough](#3-code-walkthrough)
4. [The Challenge: implement Take and Cancel](#4-the-challenge-implement-take-and-cancel)
5. [Pinocchio cheat sheet](#5-pinocchio-cheat-sheet)

---

## 1. What is an escrow?

An escrow is the "hello world" of trustless exchange. Alice has token **A** and wants token **B**. Bob has token **B** and wants token **A**. Neither wants to send first.

```
             MAKE                          TAKE
  Alice ──500 A──▶ [ Vault ] ──500 A──▶ Bob
  (maker)             ▲                   │
                      │  escrow PDA       │
                      │  records the deal │
  Alice ◀─────────────────── 100 B ───────┘
                                        (taker)

             CANCEL
  Alice ◀──500 A── [ Vault ]   (only the maker, only before a Take)
```

The program is the neutral third party. It holds Alice's tokens in a **vault** that only the program can move, remembers the terms of the deal in an **escrow account**, and releases the tokens atomically when Bob pays, or hands them back if Alice changes her mind.

Three instructions, then:

| Instruction | Who signs | What happens                                                                 | Status in this repo |
| ----------- | --------- | ---------------------------------------------------------------------------- | ------------------- |
| `Make`      | maker     | Create escrow PDA + vault, move `amount_to_give` of A into the vault         | ✅ implemented      |
| `Take`      | taker     | Taker pays `amount_to_receive` of B to maker, receives all A, accounts close | 🔨 **your job**     |
| `Cancel`    | maker     | Maker gets A back, accounts close                                            | 🔨 **your job**     |

---

## 2. Getting started

### Prerequisites

* Rust (stable), `rustup` recommended
* Solana platform tools: `cargo build-sbf` must be on your PATH. Install via the [Agave installer](https://docs.anza.xyz/cli/install), or `cargo install solana-cargo-build-sbf` and let it fetch platform-tools on first run.

### Build and test

```bash
# 1. Compile the on-chain program to target/deploy/escrow.so
cargo build-sbf

# 2. Run the LiteSVM tests (they load the .so from step 1)
cargo test -- --nocapture
```

Expected output for the shipped `Make` test:

```
Make transaction successful
CUs Consumed: ~30000
test tests::tests::test_make_instruction ... ok
```

> **Why do the SPL dev-dependencies have `features = ["no-entrypoint"]`?**
> Without it the SPL crate's own program entrypoint gets linked into the test binary and collides with the one Pinocchio generates (`duplicate symbol: entrypoint`). Keep that feature on any SPL program crate you add later.

### Project layout

```
src/
├── lib.rs                  # entrypoint, program ID, instruction dispatch
├── state/
│   └── escrow.rs           # the Escrow account layout (zero-copy)
├── instructions/
│   ├── mod.rs              # instruction enum + discriminator parsing
│   └── make.rs             # ✅ Make  (take.rs and cancel.rs go here)
└── tests/
    └── mod.rs              # LiteSVM integration tests
```

---

## 3. Code walkthrough

### 3.1 `Cargo.toml`: what we depend on

```toml
[dependencies]
pinocchio = "0.11.2"                          # core: AccountView, entrypoint, CPI helpers
pinocchio-system = "0.6.1"                    # typed CPI to the System Program (CreateAccount, ...)
pinocchio-token = "0.6.0"                     # typed CPI to SPL Token (Transfer, CloseAccount, ...)
pinocchio-associated-token-account = "0.4.0"  # typed CPI to the ATA program
pinocchio-pubkey = "0.3.0"                    # derive_address without syscall overhead
pinocchio-log = "0.5.1"                       # cheap logging

[dev-dependencies]
litesvm = "0.9.1"                             # in-process SVM, no validator needed
litesvm-token = "0.9.1"                       # helpers: CreateMint, CreateAssociatedTokenAccount, MintTo
```

These are the current crate versions as of Pinocchio 0.11. The 0.11 line changed a few signatures compared to older tutorials you may find online (accounts arrive as `&mut [AccountView]`, the token account state type is `Account`, token CPIs carry a `multisig_signers` field), so if a snippet elsewhere does not compile, check the version first.

Note the `crate-type = ["cdylib", "lib"]`. `cdylib` is what `cargo build-sbf` turns into the `.so`; `lib` is what the test module links against so it can read `crate::ID`.

### 3.2 `lib.rs`: the entrypoint

```rust
entrypoint!(process_instruction);
declare_id!("4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT");

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if program_id != &ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    // First byte = which instruction. The rest = that instruction's payload.
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match EscrowInstructions::try_from(discriminator)? {
        EscrowInstructions::Make => instructions::process_make_instruction(accounts, data)?,
        // TODO (challenge): EscrowInstructions::Take and EscrowInstructions::Cancel
        _ => return Err(ProgramError::InvalidInstructionData),   // ← Take / Cancel land here today
    }
    Ok(())
}
```

Three things to notice:

* `entrypoint!` is Pinocchio's macro. It deserializes the raw input buffer the runtime hands us into `&mut [AccountView]` **without copying**. That is where most of the CU savings over `solana-program` come from. The slice is mutable because operations that change an account in place (`set_lamports`, `close`, `try_borrow_mut`) take `&mut self` in 0.11.
* There is no Anchor-style 8-byte discriminator. We use **one byte**. Fewer bytes, cheaper transactions.
* The `_ =>` arm is where your `Take` and `Cancel` calls will go.

### 3.3 `instructions/mod.rs`: the instruction enum

```rust
pub enum EscrowInstructions {
    Make = 0,
    Take = 1,
    Cancel = 2,
    MakeV2 = 3,
}

impl TryFrom<&u8> for EscrowInstructions { /* 0 → Make, 1 → Take, 2 → Cancel, 3 → MakeV2, _ → error */ }
```

Take and Cancel already have their discriminators reserved (`1` and `2`). Your client code will put that byte first in `instruction_data`.

### 3.4 `state/escrow.rs`: zero-copy state

This is the most "Pinocchio" part of the codebase. There is no Borsh, no `serialize()`/`deserialize()`. The struct **is** the bytes.

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Escrow {
    maker: [u8; 32],
    mint_a: [u8; 32],
    mint_b: [u8; 32],
    amount_to_receive: [u8; 8],   // stored as raw LE bytes, not u64
    amount_to_give: [u8; 8],      // ↑ keeps align_of::<Escrow>() == 1
    pub bump: u8,
}

impl Escrow {
    pub const LEN: usize = core::mem::size_of::<Self>();   // 113. Let the compiler count, never hand-write the sum

    pub fn from_account_info(account_info: &mut AccountView) -> Result<&mut Self, ProgramError> {
        let mut data = account_info.try_borrow_mut()?;
        if data.len() != Escrow::LEN { return Err(ProgramError::InvalidAccountData); }
        if (data.as_ptr() as usize) % core::mem::align_of::<Self>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    pub fn amount_to_give(&self) -> u64 { u64::from_le_bytes(self.amount_to_give) }
    pub fn set_amount_to_give(&mut self, amount: u64) { self.amount_to_give = amount.to_le_bytes(); }
    // ... same pattern for every field
}
```

How to read this:

* `#[repr(C)]` pins the field order and layout so we can reinterpret the account's byte buffer as `&mut Escrow` with a pointer cast. Writing to the struct writes straight into the account.
* The `u64`s are stored as `[u8; 8]` on purpose. A real `u64` field would force 8-byte alignment, and account data buffers are not guaranteed to be aligned. Byte arrays have alignment 1, so the cast is always sound.
* The getters/setters do the little-endian conversion on the way in and out. You will use `escrow.maker()`, `escrow.mint_b()`, `escrow.amount_to_receive()` and `escrow.bump` heavily in Take and Cancel.

### 3.5 `instructions/make.rs`: the Make instruction, line by line

**Step 1: unpack accounts by position.** No names, no `#[account]` attributes. The client must pass them in exactly this order.

```rust
let [
    maker,                          // 0  signer, pays for everything
    mint_a,                         // 1  the token being deposited
    mint_b,                         // 2  the token the maker wants back
    escrow_account,                 // 3  PDA ["escrow", maker], to be created
    maker_ata,                      // 4  maker's token account for mint A (source)
    escrow_ata,                     // 5  vault: ATA(owner = escrow PDA, mint A), to be created
    system_program,                 // 6
    token_program,                  // 7
    _associated_token_program @ ..  // 8
] = accounts else {
    return Err(ProgramError::NotEnoughAccountKeys);
};
```

Because `accounts` is `&mut [AccountView]`, each binding here is a `&mut AccountView`. CPI structs want `&AccountView`, and Rust reborrows automatically, so you pass them as-is.

**Step 2: validate the maker's token account.** Anchor's `token::authority = maker, token::mint = mint_a` constraints, done by hand:

```rust
{
    let maker_ata_state = pinocchio_token::state::Account::from_account_view(maker_ata)?;
    if maker_ata_state.owner() != maker.address() { return Err(ProgramError::IllegalOwner); }
    if maker_ata_state.mint()  != mint_a.address() { return Err(ProgramError::InvalidAccountData); }
}
```

The block braces matter. `from_account_view` takes a borrow on the account's data; if that borrow is still alive when we later CPI with `maker_ata`, the runtime will refuse with a borrow error. Scoping it releases the borrow early. Remember this pattern; you will need it.

**Step 3: parse instruction data.** Layout after the discriminator byte: `[bump: u8][amount_to_receive: u64 LE][amount_to_give: u64 LE]`.

```rust
const MAKE_DATA_LEN: usize = 17;   // 1 + 8 + 8

if data.len() < MAKE_DATA_LEN {
    return Err(ProgramError::InvalidInstructionData);
}
let bump = data[0];
let amount_to_receive = u64::from_le_bytes(data[1..9].try_into().unwrap());
let amount_to_give    = u64::from_le_bytes(data[9..17].try_into().unwrap());

if !maker.is_signer() {
    return Err(ProgramError::MissingRequiredSignature);
}
```

`from_le_bytes` on a slice is the idiomatic way to read an integer out of instruction data: no alignment concerns, no `unsafe`, and the length check up front means a short payload fails cleanly instead of reading out of bounds.

**Step 4: verify the PDA.** Anchor's `seeds = [b"escrow", maker.key().as_ref()], bump` constraint. We take the bump from the client rather than calling `find_program_address` on-chain, because `find_program_address` loops over up to 255 candidates and is expensive; `derive_address` with a known bump is a single hash.

```rust
let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];
let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
if escrow_account_pda != *escrow_account.address().as_array() {
    return Err(ProgramError::InvalidSeeds);
}
```

Return an error rather than `assert_eq!`: a panic surfaces to the client as a generic "program failed to complete", while `InvalidSeeds` tells them exactly what went wrong.

**Step 5: create the escrow account, signed by the PDA.** `init` in Anchor terms.

```rust
let bump_bytes = [bump];
let seed  = [Seed::from(b"escrow"), Seed::from(maker.address().as_array()), Seed::from(&bump_bytes)];
let seeds = Signer::from(&seed);

// Refuse to overwrite an escrow that already exists for this maker.
if escrow_account.owned_by(&crate::ID) {
    return Err(ProgramError::AccountAlreadyInitialized);
}

CreateAccount {
    from: maker,
    to: escrow_account,
    lamports: Rent::get()?.try_minimum_balance(Escrow::LEN)?,
    space: Escrow::LEN as u64,
    owner: &crate::ID,
}.invoke_signed(&[seeds.clone()])?;

// Scoped so the mutable borrow on the escrow data is released before the CPIs below.
{
    let escrow_state = Escrow::from_account_info(escrow_account)?;
    escrow_state.set_maker(maker.address());
    escrow_state.set_mint_a(mint_a.address());
    escrow_state.set_mint_b(mint_b.address());
    escrow_state.set_amount_to_receive(amount_to_receive);
    escrow_state.set_amount_to_give(amount_to_give);
    escrow_state.bump = bump;
}
```

`Rent::get()?.try_minimum_balance(len)` reads the Rent sysvar and returns `(128 + len) * lamports_per_byte`. Pinocchio 0.11 follows SIMD-0194, where the sysvar's rate already includes the exemption threshold (6960 lamports per byte on mainnet). Keep that in mind when you read §3.6.

`invoke_signed` is how a program "signs" as a PDA: it proves to the runtime that it knows the seeds that produce that address. You will use exactly this `Signer` construction in Take and Cancel, except the bump will come from `escrow_state.bump` instead of instruction data.

**Step 6: create the vault.** An ATA whose *wallet* is the escrow PDA. Because the PDA is the owner, only this program (via `invoke_signed`) can ever move tokens out of it.

```rust
pinocchio_associated_token_account::instructions::Create {
    funding_account: maker,
    account: escrow_ata,
    wallet: escrow_account,
    mint: mint_a,
    token_program,
    system_program,
}.invoke()?;
```

**Step 7: deposit.** A plain SPL Token transfer, signed by the maker (who signed the transaction), so `invoke()` rather than `invoke_signed()`.

```rust
pinocchio_token::instructions::Transfer {
    from: maker_ata,
    to: escrow_ata,
    authority: maker,
    multisig_signers: &[] as &[&AccountView],   // no multisig here; the type still has to be spelled out
    amount: amount_to_give,
}.invoke()?;
```

Done. Three CPIs, ~30k CU total.

### 3.6 `tests/mod.rs`: how the test drives the program

The test uses **LiteSVM**, an in-process Solana VM. It is orders of magnitude faster than `solana-test-validator` and needs no background process.

```rust
fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    // LiteSVM 0.9 ships the pre-SIMD-0194 Rent sysvar; align it with mainnet (see below).
    svm.set_sysvar(&solana_rent::Rent { lamports_per_byte_year: 6960, exemption_threshold: 1.0, burn_percent: 50 });
    svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
    let program_data = std::fs::read("target/deploy/escrow.so").unwrap();   // ← from cargo build-sbf
    svm.add_program(program_id(), &program_data).unwrap();
    (svm, payer)
}
```

**Why the `set_sysvar` line?** Pinocchio 0.11 computes rent exemption the SIMD-0194 way: `(128 + len) * lamports_per_byte`, reading a single rate from the sysvar. Mainnet, testnet and devnet have all activated that change, so the live sysvar carries 6960. LiteSVM 0.9.1, however, still initialises the sysvar with the legacy pair (3480 lamports per byte-year, 2.0-year threshold). Without the override the program asks the System Program for exactly half the lamports the runtime requires, and `Make` fails with `InsufficientFundsForRent`. Overriding the sysvar makes the test environment match the cluster your program will actually run on. Leave that line in place for your Take and Cancel tests.

`test_make_instruction` then:

1. Creates two mints (A and B) with 6 decimals.
2. Creates the maker's ATA for A and mints 1,000 A into it.
3. Derives the escrow PDA with `Pubkey::find_program_address(&[b"escrow", maker], &PROGRAM_ID)` and the vault with `get_associated_token_address(&escrow, &mint_a)`.
4. Builds the instruction data `[0u8, bump, amount_to_receive LE, amount_to_give LE]` and the 9 `AccountMeta`s in the order from §3.5 step 1.
5. Sends the transaction and prints CU usage.
6. Reads the accounts back and asserts the vault holds 500 A, the maker's ATA dropped to 500 A, and the escrow account's 113 bytes contain the maker, both mints, both amounts and the bump.

Step 6 is the part people usually skip. A transaction that *succeeds* is not the same as a transaction that did the *right thing*. Your Take and Cancel tests should read state back the same way.

---

## 4. The Challenge: implement Take and Cancel

Your goal: make the escrow *complete*. When you are done, tokens deposited with `Make` can be either claimed by a taker who pays the asking price, or reclaimed by the maker. All accounts must be closed and rent refunded.

Work through the two instructions below. Each has the accounts, the checks, the CPIs, and the test you need to write. The code is yours to write.

### 4.1 Take (`discriminator = 1`)

**Story:** Bob sees Alice's escrow (500 A for 100 B). He calls Take. In one atomic transaction: 100 B moves from Bob to Alice, 500 A moves from the vault to Bob, the vault and escrow accounts are closed and their rent goes to Alice.

**Instruction data:** just the discriminator. Everything else is already in the escrow account.

**Accounts (suggested order):**

| #   | Account          | Writable | Signer | Notes                                                                 |
| --- | ---------------- | -------- | ------ | --------------------------------------------------------------------- |
| 0   | `taker`          | ✅       | ✅     | Pays fees; creates own ATA for A if missing                           |
| 1   | `maker`          | ✅       |        | Receives rent refunds. Must equal `escrow.maker()`                    |
| 2   | `mint_a`         |          |        | Must equal `escrow.mint_a()`                                          |
| 3   | `mint_b`         |          |        | Must equal `escrow.mint_b()`                                          |
| 4   | `escrow_account` | ✅       |        | PDA, owned by this program, will be closed                            |
| 5   | `vault`          | ✅       |        | ATA(escrow PDA, mint A), will be closed                               |
| 6   | `taker_ata_a`    | ✅       |        | ATA(taker, mint A), destination for A. May need to be created        |
| 7   | `taker_ata_b`    | ✅       |        | ATA(taker, mint B), source of B. Must exist and have enough balance  |
| 8   | `maker_ata_b`    | ✅       |        | ATA(maker, mint B), destination for B. May need to be created        |
| 9   | `system_program` |          |        |                                                                       |
| 10  | `token_program`  |          |        |                                                                       |
| 11  | `associated_token_program` |  |      |                                                                       |

**Step-by-step:**

1. **Destructure** the accounts with the same `let [ ... ] = accounts else { ... }` pattern as Make. Check `taker.is_signer()`.

2. **Load the escrow state** with `Escrow::from_account_info(escrow_account)?`. Before trusting it, verify `escrow_account.owned_by(&crate::ID)`, otherwise anyone could pass a fake account with a fake `maker`.

3. **Cross-check the passed accounts against the state.** `maker.address()` must equal `escrow.maker()`, and the two mints must match `escrow.mint_a()` / `escrow.mint_b()`. Copy out `amount_to_receive`, `bump`, and the maker address into local variables now, *then drop the escrow borrow* (end the block). You are about to CPI with `escrow_account` as a signer, and a live borrow will fail.

4. **Re-derive the PDA** with `derive_address(&[b"escrow", maker.address().as_ref(), &[bump]], None, &crate::ID.to_bytes())` and confirm it matches `escrow_account`. This is what proves the escrow belongs to *this* maker with *this* bump.

5. **Validate the vault.** Load it with `pinocchio_token::state::Account::from_account_view`, check `owner() == escrow_account.address()` and `mint() == mint_a.address()`, read `amount()` into a local (this is how much A the taker will receive), drop the borrow.

6. **Make sure the destination ATAs exist.** `taker_ata_a` and `maker_ata_b` may not have been created yet. Use `pinocchio_associated_token_account::instructions::CreateIdempotent` (safe to call if they already exist) with `taker` as the funding account. Then validate `taker_ata_b` the same way you validated `maker_ata` in Make (owner = taker, mint = mint_b).

7. **CPI #1: taker pays maker.** `pinocchio_token::instructions::Transfer { from: taker_ata_b, to: maker_ata_b, authority: taker, multisig_signers: &[] as &[&AccountView], amount: amount_to_receive }.invoke()`. Plain `invoke`, since the taker signed the transaction.

8. **Build the PDA signer.** Exactly as in Make step 5, but with `bump` read from state:
   ```rust
   let bump_bytes = [bump];
   let seed  = [Seed::from(b"escrow"), Seed::from(maker.address().as_array()), Seed::from(&bump_bytes)];
   let signer = Signer::from(&seed);
   ```

9. **CPI #2: vault pays taker.** `Transfer { from: vault, to: taker_ata_a, authority: escrow_account, multisig_signers: &[] as &[&AccountView], amount: vault_amount }.invoke_signed(&[signer.clone()])`. The vault's authority is the PDA, so this **must** be `invoke_signed`.

10. **CPI #3: close the vault.** `pinocchio_token::instructions::CloseAccount { account: vault, destination: maker, authority: escrow_account, multisig_signers: &[] as &[&AccountView] }.invoke_signed(&[signer.clone()])`. The vault's rent lamports go back to the maker, who paid for it.

11. **Close the escrow account.** The escrow is owned by *this* program, so there is no CPI. Two steps:
    * Move the lamports out first, or the runtime rejects the instruction as unbalanced:
      ```rust
      maker.set_lamports(maker.lamports() + escrow_account.lamports());
      escrow_account.set_lamports(0);
      ```
    * Then `escrow_account.close()?`. `AccountView::close` zeroes the account's data length, lamports and owner in one go. Both `set_lamports` and `close` take `&mut self`, which is why the accounts slice is mutable. `close` fails with `AccountBorrowFailed` if you still hold a borrow on the escrow data, which is another reason to copy the fields out and drop the borrow back in step 3.

12. **Wire it up.** Add `pub mod take; pub use take::*;` to `instructions/mod.rs` and the match arm in `lib.rs`.

**Test to write (`test_take_instruction`):**

Reuse the Make setup, then create a second keypair `taker`, airdrop SOL, create `taker_ata_b`, mint 100 B into it. Send the Take instruction. Then assert:

* `taker_ata_a` balance == 500 A
* `maker_ata_b` balance == 100 B
* `svm.get_account(&vault)` is `None` (or has 0 lamports / system owner)
* `svm.get_account(&escrow)` is `None` (or has 0 lamports / system owner)
* Maker's SOL balance went **up** by roughly the rent of both closed accounts

Also write **at least one negative test**: a taker who has only 50 B should fail, and Take on an escrow whose `maker` account does not match the state should fail.

### 4.2 Cancel (`discriminator = 2`)

**Story:** Alice changes her mind before anyone takes the deal. She calls Cancel. The 500 A go back to her, the vault and escrow are closed, rent is refunded to her.

**Instruction data:** just the discriminator.

**Accounts (suggested order):**

| #   | Account          | Writable | Signer | Notes                                        |
| --- | ---------------- | -------- | ------ | -------------------------------------------- |
| 0   | `maker`          | ✅       | ✅     | **Must sign.** Must equal `escrow.maker()`   |
| 1   | `mint_a`         |          |        | Must equal `escrow.mint_a()`                 |
| 2   | `escrow_account` | ✅       |        | PDA, will be closed                          |
| 3   | `vault`          | ✅       |        | Will be closed                               |
| 4   | `maker_ata_a`    | ✅       |        | Destination for the returned A               |
| 5   | `token_program`  |          |        |                                              |

**Step-by-step:**

1. Destructure. **`maker.is_signer()` is the whole security model of this instruction.** If you forget it, anyone can drain any escrow back to its maker (annoying) or, if you also forget the maker check, to themselves (catastrophic).
2. Load escrow state, verify program ownership, verify `escrow.maker() == maker.address()` and `escrow.mint_a() == mint_a.address()`. Copy out `bump`, drop the borrow.
3. Re-derive and check the PDA.
4. Validate the vault (owner = escrow PDA, mint = mint A), read its balance, drop the borrow. Validate `maker_ata_a` (owner = maker, mint = mint A).
5. Build the PDA signer.
6. `Transfer { from: vault, to: maker_ata_a, authority: escrow_account, multisig_signers: &[] as &[&AccountView], amount: vault_amount }.invoke_signed(...)`.
7. `CloseAccount { account: vault, destination: maker, authority: escrow_account, multisig_signers: &[] as &[&AccountView] }.invoke_signed(...)`.
8. Close the escrow account by hand (same as Take step 11).
9. Wire up `cancel.rs` in `mod.rs` and `lib.rs`.

You will notice steps 4 to 8 are nearly identical to Take. Feel free to factor the "drain vault + close vault + close escrow" sequence into a shared helper.

**Test to write (`test_cancel_instruction`):**

After Make, send Cancel signed by the maker. Assert the maker's ATA is back to 1,000 A and both PDA accounts are gone. Then write the negative test that matters: a **different keypair** tries to Cancel Alice's escrow and the transaction fails. If that test passes, you have a bug.

### 4.3 Definition of done

- [ ] `src/instructions/take.rs` implemented and wired into `mod.rs` + `lib.rs`
- [ ] `src/instructions/cancel.rs` implemented and wired
- [ ] `cargo build-sbf` succeeds with no new `unsafe` (the only one in the codebase is the pointer cast in `Escrow::from_account_info`)
- [ ] `cargo test` runs Make → Take (happy path), Make → Cancel (happy path), and at least the two negative tests above, all green
- [ ] A `Make → Take` round-trip costs under **50k CU** total (print `compute_units_consumed` like the Make test does)

### 4.4 Hints when you get stuck

* **"Account borrow failed" / `AccountBorrowFailed` at runtime.** You still hold a token `Account` or `Escrow` reference when you call `invoke`. Wrap the read in `{ }` and copy primitives out.
* **`Cross-program invocation with unauthorized signer`.** Your `Seed`s do not reproduce the PDA. Check: is the bump the one stored in state? Is the maker address the *maker's*, not the taker's? Is the seed literal exactly `b"escrow"`?
* **`invalid account data for instruction` from the Token program.** You are probably passing an account that is not yet initialised (forgot `CreateIdempotent`), or `from`/`to` mints do not match.
* **Where is `CloseAccount` / `CreateIdempotent`?** `pinocchio_token::instructions::CloseAccount` and `pinocchio_associated_token_account::instructions::CreateIdempotent`. If your crate version lacks one, check the docs.rs page for the version pinned in `Cargo.toml`.
* **`cannot borrow as mutable` / `types differ in mutability`.** In 0.11 the accounts slice is `&mut [AccountView]`, and `try_borrow_mut`, `set_lamports` and `close` need `&mut AccountView`. Destructure `accounts` directly (as Make does) so every binding is already mutable, and give any helper that mutates an account a `&mut AccountView` parameter.
* **`missing field multisig_signers`.** Every `pinocchio-token` 0.6 instruction struct has it. Pass `&[] as &[&AccountView]` when you are not using a multisig; the cast is needed so the generic parameter can be inferred.
* **`InsufficientFundsForRent` in a test that used to pass.** Your test's `setup()` is missing the `set_sysvar` Rent override described in §3.6.
* **Where are the lamports / close helpers?** All on `AccountView`: `lamports()`, `set_lamports(u64)`, `close()`, `resize(usize)`, `owned_by(&Address)`. They come from the `solana-account-view` crate that `pinocchio` re-exports, so search that on docs.rs if you want the full list.

---

## 5. Pinocchio cheat sheet

Quick reference for the APIs used in this repo (`pinocchio = 0.11.2`, `pinocchio-token = 0.6.0`).

| I want to…                                | Use                                                                              |
| ----------------------------------------- | -------------------------------------------------------------------------------- |
| Get an account's pubkey                   | `account.address()`                                                              |
| Check who owns an account                 | `account.owned_by(&crate::ID)` or `account.owner() == &crate::ID`                |
| Check an account signed                   | `account.is_signer()`                                                            |
| Read SPL token account fields             | `pinocchio_token::state::Account::from_account_view(acc)?` → `.owner()`, `.mint()`, `.amount()` |
| Derive a PDA (bump known)                 | `pinocchio_pubkey::derive_address(&[seed1, seed2, &[bump]], None, &crate::ID.to_bytes())` |
| Sign a CPI as a PDA                       | `let s = [Seed::from(..), ..]; let signer = Signer::from(&s); ix.invoke_signed(&[signer])` |
| Create an account                         | `pinocchio_system::instructions::CreateAccount { from, to, lamports, space, owner }` |
| Create an ATA                             | `pinocchio_associated_token_account::instructions::Create { .. }` / `CreateIdempotent { .. }` |
| Transfer SPL tokens                       | `pinocchio_token::instructions::Transfer { from, to, authority, multisig_signers, amount }` |
| Close an SPL token account                | `pinocchio_token::instructions::CloseAccount { account, destination, authority, multisig_signers }` |
| No multisig                               | `multisig_signers: &[] as &[&AccountView]`                                       |
| Rent-exempt minimum                       | `Rent::get()?.try_minimum_balance(space)?`                                       |
| Read / move lamports                      | `account.lamports()`, `account.set_lamports(n)`                                  |
| Close a program-owned account             | move lamports out, then `account.close()?`                                       |
| Log something                             | `pinocchio_log::log!("value: {}", x)`                                            |

**Account order is the API.** There is no IDL. Document the account order for every instruction in a comment at the top of its file, and keep the test in sync.

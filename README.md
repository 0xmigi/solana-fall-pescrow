# Pescrow: A Pinocchio Escrow Challenge

> **Solana Fall School · Assignment** · Native Rust on Solana with [Pinocchio](https://github.com/anza-xyz/pinocchio) 0.11.2 · Tests in Rust with LiteSVM · 3 challenges

**Finish the escrow, without a framework.**

This repo is a native Rust escrow written with Pinocchio. `Make` works and is tested. `Take` and `Cancel` are missing. Your job is to build them, and prove they work, in three challenges.

| Checkpoint | Section                                                                   | When you finish this                                   |
| ---------- | ------------------------------------------------------------------------- | ------------------------------------------------------ |
| 00         | [Setup: Fork the repo](#00-setup-fork-the-repo)                           | Your own copy, with the build tools installed          |
| 01         | [Starting point: Build and test](#01-starting-point-build-and-test)       | The shipped Make test passes on the untouched code     |
| 02         | [Read before you write: How it works](#02-read-before-you-write-how-it-works) | You can explain dispatch and zero-copy state       |
| 03         | [Read before you write: Make, line by line](#03-read-before-you-write-make-line-by-line) | You know the seven moves in `make.rs`   |
| 04         | [Challenge 1 · hard: Implement Take](#04-challenge-1--hard-implement-take) | `take.rs` compiles and is wired in                    |
| 05         | [Challenge 2 · shorter: Implement Cancel](#05-challenge-2--shorter-implement-cancel) | `cancel.rs` compiles, `MakeV2` explicitly rejected |
| 06         | [Challenge 3 · the proof: Prove it with tests](#06-challenge-3--the-proof-prove-it-with-tests) | Five tests pass, the stranger's Cancel fails |
| ?          | [When it breaks: Troubleshooting](#when-it-breaks-troubleshooting)        | Look up the error                                      |
|            | [Pinocchio cheat sheet](#pinocchio-cheat-sheet)                           | API reference for the pinned versions                  |

The same material is available as an interactive guide with progress tracking at [pinocchio-escrow-guide-3-day1.vercel.app](https://pinocchio-escrow-guide-3-day1.vercel.app).

---

## What an escrow is

Alice has token **A** and wants token **B**. Bob has B and wants A. Neither wants to send first. The program is the neutral third party: it holds Alice's A in a **vault** only the program can move, remembers the terms in an **escrow account**, and releases the tokens when Bob pays, or hands them back if Alice changes her mind.

```
             MAKE                          TAKE
  Alice ──500 A──▶ [ Vault ] ──500 A──▶ Bob
  (maker)             ▲                   │
                      │  escrow PDA       │
                      │  records the deal │
  Alice ◀─────────────────── 100 B ───────┘
                                        (taker)

             CANCEL
  Alice ◀──500 A── [ Vault ]   only the maker, only before a Take
```

| Instruction | Who signs | What happens                                                                 | Status      |
| ----------- | --------- | ---------------------------------------------------------------------------- | ----------- |
| `Make`      | maker     | Create escrow PDA + vault, move `amount_to_give` of A into the vault         | Implemented |
| `Take`      | taker     | Taker pays `amount_to_receive` of B to maker, receives all A, accounts close | **Your job** |
| `Cancel`    | maker     | Maker gets A back, accounts close                                            | **Your job** |

### Why Pinocchio, not Anchor

If you have only written Anchor programs, this is where you see what the framework was doing for you: manual account validation, manual PDA derivation, zero-copy state, and raw CPIs. The payoff shows up in compute: the Make instruction here runs in about 30k CU.

### The files you will touch

| File                      | What it is                                                                 |
| ------------------------- | -------------------------------------------------------------------------- |
| `lib.rs`                  | Entrypoint and instruction dispatch. Add two match arms.                   |
| `instructions/mod.rs`     | The instruction enum. Register your new modules.                           |
| `instructions/make.rs`    | Finished. Read it first, copy its patterns. Checkpoint 03.                 |
| `instructions/take.rs`    | Does not exist yet. Challenge 1.                                           |
| `instructions/cancel.rs`  | Does not exist yet. Challenge 2.                                           |
| `state/escrow.rs`         | The zero-copy account layout. Read only, you will call its getters.        |
| `tests/mod.rs`            | LiteSVM tests. One exists; you add four. Challenge 3.                      |

Search the code for `TODO (challenge)`. That comment in `lib.rs` marks where Take and Cancel plug in.

> **This repo is on Pinocchio 0.11. Older tutorials will not compile.**
>
> The 0.11 line changed signatures that almost every escrow tutorial online still shows the old way. Three to know before you start:
>
> 1. Accounts arrive as `&mut [AccountView]`, not `&[AccountView]`.
> 2. The token account state type is `pinocchio_token::state::Account`, not `TokenAccount`.
> 3. Token CPI structs carry a `multisig_signers` field.
>
> If a snippet from elsewhere does not compile, check its version before you change your own code. The cheat sheet at the end of this page is written for the versions pinned in this repo.

---

## 00 · Setup: Fork the repo

**When you finish this:** your own copy of the assignment on GitHub and on your machine, with the Solana build tools installed.

1. Open [github.com/decentra1ized/solana-fall-pescrow](https://github.com/decentra1ized/solana-fall-pescrow) and click **Fork**, top right.
2. Clone *your fork*, not the original. Put your GitHub username in the link.

```bash
git clone https://github.com/YOUR-USERNAME/solana-fall-pescrow.git
cd solana-fall-pescrow
```

### Tools this repo expects

| Tool              | Version   | How to get it                                                                                                              |
| ----------------- | --------- | -------------------------------------------------------------------------------------------------------------------------- |
| Rust              | stable    | `rustup update stable`                                                                                                     |
| `cargo build-sbf` | Agave 3.x | Comes with the [Agave installer](https://docs.anza.xyz/cli/install). Or `cargo install solana-cargo-build-sbf` and let it fetch platform-tools on first run |
| Anchor            |           | Not used. This is native Rust                                                                                              |
| Node, Yarn        |           | Not used. Tests are in Rust                                                                                                |

### Crates this repo pins

| Crate                                | Version | What it gives you                                                        |
| ------------------------------------ | ------- | ------------------------------------------------------------------------ |
| `pinocchio`                          | 0.11.2  | `AccountView`, `entrypoint!`, CPI helpers, sysvars                       |
| `pinocchio-system`                   | 0.6.1   | Typed CPI to the System Program: `CreateAccount`, …                      |
| `pinocchio-token`                    | 0.6.0   | Typed CPI to SPL Token: `Transfer`, `CloseAccount`, …                    |
| `pinocchio-associated-token-account` | 0.4.0   | Typed CPI to the ATA program                                             |
| `pinocchio-pubkey`                   | 0.3.0   | `derive_address`, now a published crate rather than a git dependency     |
| `litesvm`, `litesvm-token`           | 0.9.1   | The in-process SVM and its mint / ATA helpers                            |
| `solana-rent`                        | 3.1.0   | Dev-only. The `Rent` type the tests use to override the sysvar           |

> **No validator either.** The tests run in LiteSVM, an in-process Solana VM. There is no `solana-test-validator` to start, and nothing to deploy. Build the `.so`, run the tests, done.

Push after each challenge, so your progress is saved:

```bash
git add -A && git commit -m "challenge 1: take" && git push
```

---

## 01 · Starting point: Build and test

**When you finish this:** the shipped Make test passes on the untouched code. Prove this before changing anything.

```bash
# 1. Compile the on-chain program to target/deploy/escrow.so
cargo build-sbf

# 2. Run the LiteSVM tests (they load the .so from step 1)
cargo test -- --nocapture
```

You should see one passing test and its compute usage:

```
Make transaction successful
CUs Consumed: ~30000
test tests::tests::test_make_instruction ... ok
```

> **Always build before you test.** The tests do not compile your program. They read the finished `target/deploy/escrow.so` into LiteSVM. Skip `cargo build-sbf` and the tests run your *old* program, not your latest change.
>
> `cargo test` alone compiles the crate for your laptop's CPU, which is what the test binary needs. It does not produce a `.so`.

<details>
<summary><b>Why the SPL dev-dependencies have <code>no-entrypoint</code></b></summary>

Without it the SPL crate's own program entrypoint gets linked into the test binary and collides with the one Pinocchio generates (`duplicate symbol: entrypoint`). Keep that feature on any SPL program crate you add later.

</details>

---

## 02 · Read before you write: How it works

**When you finish this:** you can explain how an instruction reaches its handler, and how the escrow account stores its data without any serialization library.

### 1 · `lib.rs`, the entrypoint

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

* `entrypoint!` is Pinocchio's macro. It parses the runtime input into `&mut [AccountView]` without the allocations and copies of the standard SDK path. Combined with explicit validation and compact instruction data, that can significantly reduce compute.
* The slice is mutable because in 0.11 everything that changes an account in place (`set_lamports`, `close`, `try_borrow_mut`) takes `&mut self`. Your `take.rs` and `cancel.rs` handlers must take `&mut [AccountView]` too.
* There is no Anchor-style 8-byte discriminator. We use one byte, so the instruction data stays smaller.
* The `_ =>` arm is where your Take and Cancel calls will go.

### 2 · `instructions/mod.rs`, the enum

```rust
pub enum EscrowInstructions {
    Make = 0,
    Take = 1,
    Cancel = 2,
    MakeV2 = 3,
}

impl TryFrom<&u8> for EscrowInstructions { /* 0 → Make, 1 → Take, 2 → Cancel, 3 → MakeV2, _ → error */ }
```

Take and Cancel already have their discriminators reserved, `1` and `2`. Your test code will put that byte first in `instruction_data`.

### 3 · `state/escrow.rs`, zero-copy state

This is the most "Pinocchio" part of the codebase. There is no Borsh, no `serialize()` or `deserialize()`. The struct **is** the bytes.

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

* `#[repr(C)]` pins the field order and layout so we can reinterpret the account's byte buffer as `&mut Escrow` with a pointer cast. Writing to the struct writes straight into the account.
* The `u64`s are stored as `[u8; 8]` on purpose. A real `u64` field would force 8-byte alignment, and account data buffers are not guaranteed to be aligned. Byte arrays have alignment 1, so the cast is always sound.
* The getters and setters do the little-endian conversion on the way in and out. You will use `escrow.maker()`, `escrow.mint_b()`, `escrow.amount_to_receive()` and `escrow.bump` heavily in Take and Cancel.

> **A borrow you must remember to drop.** `from_account_info` calls `try_borrow_mut()` on the account. While that `&mut Escrow` is alive, any CPI that touches the same account fails with `AccountBorrowFailed`. Read what you need into locals, then let the reference go out of scope before you invoke anything. This will bite you in Take.

---

## 03 · Read before you write: Make, line by line

**When you finish this:** you know the seven moves in `make.rs`. Take and Cancel reuse five of them.

### 1 · Unpack accounts by position

No names, no `#[account]` attributes. The client must pass them in exactly this order.

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

### 2 · Validate the maker's token account

Anchor's `token::authority = maker, token::mint = mint_a` constraints, done by hand:

```rust
{
    let maker_ata_state = pinocchio_token::state::Account::from_account_view(maker_ata)?;
    if maker_ata_state.owner() != maker.address() { return Err(ProgramError::IllegalOwner); }
    if maker_ata_state.mint()  != mint_a.address() { return Err(ProgramError::InvalidAccountData); }
}
```

> **The braces matter.** `from_account_view` borrows the account's data. If that borrow is still alive when we later CPI with `maker_ata`, the runtime refuses with a borrow error. Scoping it releases the borrow early. You will use this block pattern at least four times in Take.

### 3 · Parse instruction data

Layout after the discriminator byte: `[bump: u8][amount_to_receive: u64 LE][amount_to_give: u64 LE]`.

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

### 4 · Verify the PDA

Anchor's `seeds = [b"escrow", maker.key().as_ref()], bump` constraint. We take the bump from the client rather than calling `find_program_address` on-chain, because that loops over up to 255 candidates. `derive_address` with a known bump is a single hash.

```rust
let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];
let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
if escrow_account_pda != *escrow_account.address().as_array() {
    return Err(ProgramError::InvalidSeeds);
}
```

Return an error rather than `assert_eq!`. A panic surfaces to the client as a generic "program failed to complete", while `InvalidSeeds` says exactly what went wrong.

### 5 · Create the escrow account, signed by the PDA

`init` in Anchor terms.

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

Note the shape: an early return on the "already exists" case, then the happy path straight down the function. Guard clauses read better than nesting the whole instruction inside an `if`, and `AccountAlreadyInitialized` tells the client more than `IllegalOwner` would.

> **This `Signer` is the whole trick.** `invoke_signed` is how a program "signs" as a PDA: it proves to the runtime that it knows the seeds that produce that address. You will build exactly this `Signer` in Take and Cancel, except the bump comes from `escrow_state.bump` instead of instruction data.

> **How much rent the account needs.** `Rent::get()?.try_minimum_balance(len)` returns `(128 + len) * lamports_per_byte`. Pinocchio 0.11 follows SIMD-0194, which folded the old 2.0-year exemption threshold into the rate: one integer multiply, no floating point, 8 CU instead of ~256.
>
> Its `Rent` struct is 8 bytes. It reads only the rate and ignores the sysvar's remaining fields. That detail is what makes the test setup below need one extra line.

### 6 · Create the vault

An ATA whose *wallet* is the escrow PDA. Because the PDA is the owner, only this program, via `invoke_signed`, can ever move tokens out of it.

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

### 7 · Deposit

A plain SPL Token transfer, signed by the maker who signed the transaction. So `invoke()`, not `invoke_signed()`.

```rust
pinocchio_token::instructions::Transfer {
    from: maker_ata,
    to: escrow_ata,
    authority: maker,
    multisig_signers: &[] as &[&AccountView],   // no multisig here; the type still has to be spelled out
    amount: amount_to_give,
}.invoke()?;
```

`multisig_signers` is new in `pinocchio-token` 0.6 and there is no `Default` to lean on, so every `Transfer` and `CloseAccount` you write needs it. The `as &[&AccountView]` cast is there so the generic parameter can be inferred from an empty slice.

Done. Three CPIs, about 30k CU total.

### How the test drives it

`setup()` creates a LiteSVM, overrides the Rent sysvar, airdrops a payer, and loads `target/deploy/escrow.so`. Then `test_make_instruction`:

1. Creates two mints, A and B, with 6 decimals.
2. Creates the maker's ATA for A and mints 1,000 A into it.
3. Derives the escrow PDA with `Pubkey::find_program_address(&[b"escrow", maker], &PROGRAM_ID)` and the vault with `get_associated_token_address(&escrow, &mint_a)`.
4. Builds the instruction data `[0u8, bump, amount_to_receive LE, amount_to_give LE]` and the 9 `AccountMeta`s in the order from step 1.
5. Sends the transaction and prints CU usage.
6. Reads the accounts back and asserts the vault holds 500 A, the maker's ATA dropped to 500 A, and the escrow's 113 bytes contain the maker, both mints, both amounts and the bump.

> **Step 6 is the part people skip.** A transaction that *succeeds* is not the same as a transaction that did the *right thing*. Your Take and Cancel tests must read state back the same way.

**The one line in `setup()` you must not delete:**

```rust
let mut svm = LiteSVM::new();

// LiteSVM 0.9 still ships the pre-SIMD-0194 Rent sysvar. Match the live cluster.
#[allow(deprecated)]
svm.set_sysvar(&solana_rent::Rent {
    lamports_per_byte_year: 6960,
    exemption_threshold: 1.0,
    burn_percent: 50,
});
```

<details>
<summary><b>Why, and what breaks without it</b></summary>

Mainnet, testnet and devnet have activated SIMD-0194, so the live sysvar carries the folded rate: 6960 lamports per byte with a threshold of 1.0. LiteSVM 0.9.1 still initialises the sysvar with the legacy pair, 3480 per byte-year and a 2.0 threshold. The same product, split differently.

Pinocchio 0.11 reads only the rate and multiplies once. Against the legacy sysvar it therefore asks the System Program for exactly half the lamports the runtime requires, and Make fails with `InsufficientFundsForRent`. Overriding the sysvar makes both sides agree on 6960. Carry that line into every helper you factor out for Take and Cancel.

</details>

---

## 04 · Challenge 1 · hard: Implement Take

**Goal:** Bob sees Alice's escrow, 500 A for 100 B, and calls Take. In one atomic transaction, 100 B moves from Bob to Alice, 500 A moves from the vault to Bob, and the vault and escrow accounts are closed with their rent refunded to Alice.

**Why it matters:** without Take, the escrow is a one-way deposit box. This is the instruction that makes it a trade.

**Instruction data:** just the discriminator, `1`. Everything else is already in the escrow account.

**Accounts, suggested order:**

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

> **Account order is the API.** There is no IDL. Write the account order in a comment at the top of `take.rs`, and keep the test in sync with it.

### 1 · Destructure and check the signer

Your handler takes `accounts: &mut [AccountView]`, same as `process_make_instruction`. Destructure with the same `let [ ... ] = accounts else { ... }` pattern, with the twelve accounts above, so every binding is already a `&mut AccountView`. Then check `taker.is_signer()`.

### 2 · Load the escrow state, and trust it only after checking the owner

`Escrow::from_account_info(escrow_account)?` gives you the state. Before trusting anything in it, verify `escrow_account.owned_by(&crate::ID)`. Otherwise anyone could pass a fake account with a fake `maker`.

### 3 · Cross-check the passed accounts against the state

`maker.address()` must equal `escrow.maker()`, and the two mints must match `escrow.mint_a()` and `escrow.mint_b()`. Copy `amount_to_receive` and `bump` into locals now, then end the block so the borrow drops. You are about to CPI with `escrow_account` as a signer, and a live borrow will fail.

```rust
let (amount_to_receive, bump) = {
    let escrow = Escrow::from_account_info(escrow_account)?;
    if escrow.maker()  != *maker.address()  { return Err(ProgramError::InvalidAccountData); }
    if escrow.mint_a() != *mint_a.address() { return Err(ProgramError::InvalidAccountData); }
    if escrow.mint_b() != *mint_b.address() { return Err(ProgramError::InvalidAccountData); }
    (escrow.amount_to_receive(), escrow.bump)
};   // ← borrow released here
```

### 4 · Re-derive the PDA

`derive_address(&[b"escrow", maker.address().as_ref(), &[bump]], None, &crate::ID.to_bytes())` must equal `escrow_account.address()`. This is what proves the escrow belongs to *this* maker with *this* bump.

### 5 · Validate the vault

Load it with `pinocchio_token::state::Account::from_account_view`, check `owner() == escrow_account.address()` and `mint() == mint_a.address()`, read `amount()` into a local. That is how much A the taker will receive. Drop the borrow.

### 6 · Make sure the destination ATAs exist

`taker_ata_a` and `maker_ata_b` may not have been created yet. Use `pinocchio_associated_token_account::instructions::CreateIdempotent`, which is safe to call if they already exist, with `taker` as the funding account. Then validate `taker_ata_b` the same way Make validated `maker_ata`: owner = taker, mint = mint_b.

<details>
<summary>Hint: <code>CreateIdempotent</code> has the same shape as <code>Create</code></summary>

```rust
pinocchio_associated_token_account::instructions::CreateIdempotent {
    funding_account: taker,
    account: taker_ata_a,
    wallet: taker,
    mint: mint_a,
    system_program,
    token_program,
}.invoke()?;
// and again for maker_ata_b with wallet: maker, mint: mint_b
```

</details>

> **From here on, write it before you open the hint.** The remaining steps give you the requirement, not the code. Each one has the finished call behind a collapsed hint. Try it from the cheat sheet and from `make.rs` first; open the hint when you are stuck, not before.

### 7 · CPI #1, taker pays maker

Transfer `amount_to_receive` of mint B from `taker_ata_b` to `maker_ata_b`. The authority is the taker, who signed the transaction, so this is a plain `invoke()`.

<details>
<summary>Hint: the transfer call</summary>

```rust
pinocchio_token::instructions::Transfer {
    from: taker_ata_b,
    to: maker_ata_b,
    authority: taker,
    multisig_signers: &[] as &[&AccountView],
    amount: amount_to_receive,
}.invoke()?;
```

</details>

### 8 · Build the PDA signer, then CPI #2, vault pays taker

The vault's authority is the escrow PDA, not any wallet, so this transfer must be `invoke_signed` with a `Signer` built from the same three seeds Make used: `b"escrow"`, the maker's address, and the bump. The bump comes from state, not from instruction data.

Move `vault_amount`, the balance you read in step 5, not `amount_to_give` from state, out of the vault and into `taker_ata_a`.

<details>
<summary>Hint: signing as the PDA</summary>

```rust
let bump_bytes = [bump];
let seed   = [Seed::from(b"escrow"), Seed::from(maker.address().as_array()), Seed::from(&bump_bytes)];
let signer = Signer::from(&seed);

pinocchio_token::instructions::Transfer {
    from: vault,
    to: taker_ata_a,
    authority: escrow_account,
    multisig_signers: &[] as &[&AccountView],
    amount: vault_amount,
}.invoke_signed(&[signer.clone()])?;
```

</details>

### 9 · CPI #3, close the vault

An empty token account still holds rent. Close it with the token program's `CloseAccount`, again signed by the PDA, and send the lamports to the maker, who paid for it.

### 10 · Close the escrow account, by hand

The escrow is owned by *this* program, so there is no CPI for it. Two moves, in this order: add its lamports to the maker's balance and zero its own, then call `close()`. Skip the lamport move and the runtime rejects the instruction as unbalanced.

`AccountView::close` zeroes the account's data length, lamports and owner in one go. Both `set_lamports` and `close` take `&mut self`, which is why the accounts slice is mutable. `close` fails with `AccountBorrowFailed` if you still hold a borrow on the escrow data, which is another reason to copy the fields out in step 3.

<details>
<summary>Hint: closing both accounts</summary>

```rust
pinocchio_token::instructions::CloseAccount {
    account: vault,
    destination: maker,
    authority: escrow_account,
    multisig_signers: &[] as &[&AccountView],
}.invoke_signed(&[signer.clone()])?;

maker.set_lamports(maker.lamports() + escrow_account.lamports());
escrow_account.set_lamports(0);
escrow_account.close()?;
```

</details>

### 11 · Wire it up

1. In `instructions/mod.rs`: `pub mod take; pub use take::*;`
2. In `lib.rs`, replace the `_ =>` arm with a real one for `EscrowInstructions::Take`.
3. `cargo build-sbf`. It must compile before you touch the tests.

> **You are done when** `cargo build-sbf` succeeds and the account order comment at the top of `take.rs` matches the table above. The tests come in Challenge 3.

---

## 05 · Challenge 2 · shorter: Implement Cancel

**Goal:** Alice changes her mind before anyone takes the deal. She calls Cancel. The 500 A go back to her, the vault and escrow are closed, and rent is refunded to her.

**Why it matters:** without Cancel, a deposit nobody takes is locked forever. With a *wrong* Cancel, anyone can drain any escrow. This is the shortest instruction and the easiest to get dangerously wrong.

**Instruction data:** just the discriminator, `2`.

**Accounts, suggested order:**

| #   | Account          | Writable | Signer | Notes                                        |
| --- | ---------------- | -------- | ------ | -------------------------------------------- |
| 0   | `maker`          | ✅       | ✅     | **Must sign.** Must equal `escrow.maker()`   |
| 1   | `mint_a`         |          |        | Must equal `escrow.mint_a()`                 |
| 2   | `escrow_account` | ✅       |        | PDA, will be closed                          |
| 3   | `vault`          | ✅       |        | Will be closed                               |
| 4   | `maker_ata_a`    | ✅       |        | Destination for the returned A               |
| 5   | `token_program`  |          |        |                                              |

> **`maker.is_signer()` is the critical authorization check.** Forget it, and anyone can drain any escrow back to its maker, which is annoying. Forget the stored-maker check too, and they can drain it to themselves, which is catastrophic.
>
> The stored maker and the PDA re-derivation still matter. `is_signer` proves who sent the transaction; those two prove it is the right escrow for that signer.

### The steps

1. Destructure. Check `maker.is_signer()`.
2. Load escrow state, verify program ownership, verify `escrow.maker() == maker.address()` and `escrow.mint_a() == mint_a.address()`. Copy out `bump`, drop the borrow.
3. Re-derive and check the PDA.
4. Validate the vault: owner = escrow PDA, mint = mint A. Read its balance, drop the borrow. Validate `maker_ata_a`: owner = maker, mint = mint A.
5. Build the PDA signer.
6. `Transfer { from: vault, to: maker_ata_a, authority: escrow_account, multisig_signers: &[] as &[&AccountView], amount: vault_amount }.invoke_signed(...)`
7. `CloseAccount { account: vault, destination: maker, authority: escrow_account, multisig_signers: &[] as &[&AccountView] }.invoke_signed(...)`
8. Close the escrow account by hand, same as Take step 10.
9. Wire up `cancel.rs` in `mod.rs` and `lib.rs`. Build.

<details>
<summary>Hint: steps 4 to 8 look familiar</summary>

They are the tail end of Take. Consider extracting "drain vault to a destination, close vault, close escrow" into a helper in `src/instructions/shared.rs` that takes the destination ATA and the PDA `Signer`. Both instructions then become validation plus one call. Any helper that mutates an account (`set_lamports`, `close`) must take `&mut AccountView`.

</details>

> **You are done when** `cargo build-sbf` succeeds with both new match arms in `lib.rs`, and there is no `_ =>` arm silently swallowing `MakeV2`. Return an explicit error for it.

---

## 06 · Challenge 3 · the proof: Prove it with tests

**Goal:** four new LiteSVM tests. Two happy paths that read state back, and two negative tests that must fail for the right reason.

**Why it matters:** a Take that "works" but leaves the escrow open, or a Cancel that a stranger can call, both pass a test that only checks the transaction succeeded.

### 1 · Factor the Make setup into a helper

Every test starts the same way: two mints, a funded maker ATA, a Make transaction. Pull that out of `test_make_instruction` into a function that returns what the next steps need: the `svm`, the maker, both mints, the escrow PDA and bump, and the vault address.

> **Keep the Rent override.** Your helper must still call `svm.set_sysvar` the way `setup()` does, or every test you write from here fails at the Make step with `InsufficientFundsForRent`. Reuse `setup()` rather than building a fresh LiteSVM by hand.

### 2 · `test_take_instruction`

1. Run the Make helper.
2. Create a second keypair, `taker`. Airdrop it SOL. Create `taker_ata_b` and mint 100 B into it.
3. Derive `taker_ata_a` and `maker_ata_b` with `get_associated_token_address`. Do *not* create them. Your program does that with `CreateIdempotent`.
4. Build the instruction: data `vec![1u8]`, the twelve `AccountMeta`s in the order from Challenge 1, signed by the taker.
5. Send it, print `compute_units_consumed`.

Then assert:

| Read back                  | Expect                                                |
| -------------------------- | ----------------------------------------------------- |
| `taker_ata_a.amount`       | 500 A                                                 |
| `maker_ata_b.amount`       | 100 B                                                 |
| `svm.get_account(&vault)`  | `None`, or 0 lamports and system owner                |
| `svm.get_account(&escrow)` | `None`, or 0 lamports and system owner                |
| maker SOL balance          | Went up by roughly the rent of both closed accounts   |

### 3 · `test_cancel_instruction`

After Make, send Cancel signed by the maker: data `vec![2u8]`, the six accounts from Challenge 2. Assert the maker's ATA is back to 1,000 A and both PDA accounts are gone.

### 4 · Two negative tests

| Test                                        | Expect             | What it proves                                                                     |
| ------------------------------------------- | ------------------ | ---------------------------------------------------------------------------------- |
| Take by a taker who has only 50 B           | Transaction fails  | The token program rejects the underfunded transfer, and nothing else moved         |
| Cancel signed by a different keypair        | Transaction fails  | The maker check and the signer check both hold                                     |

> **If the stranger's Cancel passes, you have a bug.** This is the one test that matters most in the whole assignment. Do not `unwrap()` the send here; assert it returns `Err`. Then read the vault back and confirm the 500 A are still there.

<details>
<summary>Hint: asserting a failed transaction in LiteSVM</summary>

```rust
let result = svm.send_transaction(tx);
assert!(result.is_err(), "a stranger must not be able to cancel someone else's escrow");

let vault_acc = svm.get_account(&vault).unwrap();
let vault_state = spl_token_2022::state::Account::unpack(&vault_acc.data).unwrap();
assert_eq!(vault_state.amount, 500_000_000);   // A never left
```

</details>

<details>
<summary>Hint: Take on a mismatched maker should fail too</summary>

Optional fifth negative test: send a Take where the `maker` account does not match `escrow.maker()`. The cross-check in Take step 3 must reject it before any token moves. Same assertion pattern as above.

</details>

### 5 · Definition of done

- [ ] `src/instructions/take.rs` implemented and wired into `mod.rs` and `lib.rs`.
- [ ] `src/instructions/cancel.rs` implemented and wired.
- [ ] `cargo build-sbf` succeeds with no new `unsafe`. The only one in the codebase is the pointer cast in `Escrow::from_account_info`; you should not need another.
- [ ] `cargo test` runs Make → Take, Make → Cancel, and at least the two negative tests above, all green.
- [ ] Every test prints `compute_units_consumed`, like the Make test does, and you have compared Take and Cancel against Make's ~30k. There is no fixed budget to hit, but a number far above Make's is worth a look at how many CPIs you are making.

> **You are done when** five tests pass, the stranger's Cancel fails, and you can say what Take and Cancel cost next to Make. Push it.

---

## When it breaks: Troubleshooting

<details>
<summary><code>Failed to read program SO file … Run `cargo build-sbf` first</code></summary>

The tests load `target/deploy/escrow.so` and it is not there. Run `cargo build-sbf`, then `cargo test`.

</details>

<details>
<summary>My change does nothing in the tests</summary>

You edited the program but did not rebuild the `.so`. `cargo test` compiles the crate for your CPU, not for SBF. Run `cargo build-sbf` again; the tests pick up the new binary.

</details>

<details>
<summary><code>InsufficientFundsForRent</code> in a test</summary>

Your test setup lost the Rent sysvar override. Pinocchio 0.11 computes rent the SIMD-0194 way and LiteSVM 0.9.1 seeds the legacy sysvar, so the program asks for half the required lamports. Keep the `svm.set_sysvar(...)` line from `setup()` in every helper. See "How the test drives it" in checkpoint 03.

</details>

<details>
<summary><code>Account borrow failed</code> / <code>AccountBorrowFailed</code></summary>

A `Ref` on the account's data (from `Account::from_account_view` or `Escrow::from_account_info`) is still alive when you CPI or call `close()`. Wrap the read in `{ }`, copy primitives out, and let the borrow drop before you invoke anything.

</details>

<details>
<summary><code>cannot borrow as mutable</code>, or <code>types differ in mutability</code></summary>

In 0.11 the accounts slice is `&mut [AccountView]`, and `try_borrow_mut`, `set_lamports` and `close` need `&mut AccountView`. Destructure `accounts` directly, as Make does, so every binding is already mutable, and give any helper that mutates an account a `&mut AccountView` parameter. Your handler signature must be `accounts: &mut [AccountView]`.

</details>

<details>
<summary><code>missing field `multisig_signers`</code></summary>

Every `pinocchio-token` 0.6 instruction struct has it and there is no `Default`. Pass `multisig_signers: &[] as &[&AccountView]` when you are not using a multisig; the cast lets the generic parameter be inferred from an empty slice.

</details>

<details>
<summary><code>cannot find type `TokenAccount` in `pinocchio_token::state`</code></summary>

You copied from an older tutorial. In `pinocchio-token` 0.6 the type is `pinocchio_token::state::Account`.

</details>

<details>
<summary><code>Cross-program invocation with unauthorized signer</code></summary>

Your `Seed`s do not reproduce the PDA. Check three things: the bump is the one stored in state, the address is the *maker's* (not the taker's), and the literal is exactly `b"escrow"`.

</details>

<details>
<summary><code>invalid account data for instruction</code>, from the Token program</summary>

You are passing an account that is not initialised yet (forgot `CreateIdempotent` for `taker_ata_a` or `maker_ata_b`), or the `from` and `to` accounts have different mints.

</details>

<details>
<summary><code>InvalidInstructionData</code> when I send Take</summary>

The `_ =>` arm in `lib.rs` is still catching discriminator `1`. Add the real match arm for `EscrowInstructions::Take`, then `cargo build-sbf`.

</details>

<details>
<summary><code>NotEnoughAccountKeys</code></summary>

Your test passes fewer accounts than the handler destructures. Count against the table: Take needs 12, Cancel needs 6.

</details>

<details>
<summary><code>sum of account balances before and after instruction do not match</code></summary>

You called `close()` on the escrow (or let the vault close) without moving its lamports first. Do the `set_lamports` pair, then `close()`. Lamports can never disappear inside an instruction; they have to land somewhere.

</details>

<details>
<summary><code>duplicate symbol: entrypoint</code></summary>

An SPL program crate in `[dev-dependencies]` is missing `features = ["no-entrypoint"]`, so its entrypoint collides with the one Pinocchio generates. Add the feature to the crate you just added.

</details>

<details>
<summary>Do I need an <code>unsafe</code> block to check the owner?</summary>

No. In 0.11 `AccountView::owner()` is a safe fn, and `owned_by(&crate::ID)` is the shortest way to write the check. The only `unsafe` in this codebase is the pointer cast inside `Escrow::from_account_info`.

</details>

<details>
<summary>Where is <code>CloseAccount</code> / <code>CreateIdempotent</code> / <code>set_lamports</code>?</summary>

`pinocchio_token::instructions::CloseAccount`, `pinocchio_associated_token_account::instructions::CreateIdempotent`, and `set_lamports`, `lamports`, `close`, `owned_by` are methods on `AccountView` (from the `solana-account-view` crate that `pinocchio` re-exports). If your IDE cannot find one, check that `Cargo.toml` still pins the versions in the table at the top.

</details>

---

## Pinocchio cheat sheet

Quick reference for the APIs used in this repo: `pinocchio = 0.11.2`, `pinocchio-token = 0.6.0`.

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
| Say "no multisig"                         | `multisig_signers: &[] as &[&AccountView]`                                       |
| Rent-exempt minimum                       | `Rent::get()?.try_minimum_balance(space)?` → `(128 + space) * lamports_per_byte` |
| Read / move lamports                      | `account.lamports()`, `account.set_lamports(n)`                                  |
| Close a program-owned account             | move lamports out, then `account.close()?`                                       |
| Log something                             | `pinocchio_log::log!("value: {}", x)`                                            |

**Account order is the API.** There is no IDL. Document the account order for every instruction in a comment at the top of its file, and keep the test in sync.

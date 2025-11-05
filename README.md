# Anchor-NFTFlashLoan
A Solana Anchor-based program implementing an **NFT flash loan** system with integrated escrow and repayment enforcement.
## Overview
`anchor-nftflashloan` is a secure, on-chain **flash loan protocol for NFTs** built using the **Anchor framework** on Solana. It enables any user (the **borrower**) to borrow an NFT for the duration of a single transaction, use it in DeFi protocols (e.g., staking, lending, liquidations), and return it — **all atomically**. If the NFT is not returned or repayment conditions are unmet, the transaction reverts.

The program uses an **escrow vault** to hold the NFT during the loan, invokes user-defined callback logic via CPI, and enforces repayment through programmable checks.

## Features
- **Instant NFT Borrowing**: Borrow any SPL NFT in a single transaction.
- **Callback Execution**: Borrower provides a callback instruction to execute arbitrary logic with the borrowed NFT.
- **Atomic Repayment**: NFT must be returned to the vault before transaction ends.
- **Flexible Fees (Optional)**: Configurable flash loan fee (in SOL or tokens) paid to liquidity providers.
- **Event Emissions**: Full audit trail with `BorrowEvent`, `RepayEvent`, and `FeeCollectedEvent`.
- **Secure Escrow PDA**: Uses seeded PDAs with bump for vault isolation.

## How It Works
1. **Initialize Flash Loan Vault**  
   Liquidity provider deposits NFT into a program-controlled escrow vault.

2. **Borrow (Flash Loan)**  
   Borrower calls `borrow`, specifying:
   - Target NFT vault
   - Callback program + instruction data  
   The program:
   - Transfers NFT from vault → borrower’s temporary ATA
   - Invokes borrower’s callback (CPI)
   - Checks NFT is back in vault post-callback

3. **Repayment & Closure**  
   - If NFT is returned → loan succeeds, optional fee deducted
   - If not returned → transaction reverts (flash loan fails safely)

### Main Data Structure
```rust
#[account]
pub struct FlashLoanVault {
    pub owner: Pubkey,           // Liquidity provider
    pub mint: Pubkey,            // NFT mint address
    pub bump: u8,                // PDA bump
    pub fee_basis_points: u16,   // Optional fee (e.g., 10 = 0.1%)
    pub is_active: bool,         // Vault status
}
````

## Usage
### Clone the Repo
```bash
git clone https://github.com/akshxdevs/anchor-nftflashloan.git
cd anchor-nftflashloan
Install Dependencies
bashyarn install
Build the Project
bashanchor build
Test the Project
bashanchor test
````
# **FlashLoanVault: NFT Flash Loans on Solana**

**Flash loans for NFTs — trustless, instant, and fully on-chain.**  
Atomic borrowing of NFTs with zero collateral. Perfect for DeFi composability, MEV, and cross-protocol interactions.

---

## **Example Flow**

1. **Provider deposits NFT** into `FlashLoanVault` via `deposit_nft`.
2. **Borrower calls `borrow`:**
   - Specifies vault
   - Provides callback (e.g., stake NFT in another protocol)
3. **Callback executes** with borrowed NFT.
4. **NFT auto-returned** to vault via `repay_nft` in same tx.
5. **Fee (if any)** transferred to provider.
6. **Events emitted** for indexing.

> **If repayment fails → entire transaction reverts. No risk to lender.**

---

## **Key Files**
programs/anchor-nftflashloan/src/lib.rs
text> Core program entrypoint and context definitions.
programs/anchor-nftflashloan/src/instructions/
text- `deposit_nft.rs`  
- `borrow.rs`  
- `repay_nft.rs`  
- `withdraw_nft.rs`
programs/anchor-nftflashloan/src/state.rs
text> `FlashLoanVault` and event structs.
tests/anchor-nftflashloan.ts
text> Full integration tests using mock callback programs.

---

## **Events**

| Event               | Description |
|---------------------|-----------|
| **`BorrowEvent`**     | Emitted when flash loan starts (includes borrower, mint, timestamp) |
| **`RepayEvent`**      | Emitted on successful repayment |
| **`FeeCollectedEvent`** | Tracks fee amount and recipient |

---

## **Requirements**

- **Node.js ≥ 18**
- **Yarn**
- **Solana CLI**
- **Anchor CLI**  
  ```bash
  avm install latest && avm use latest

Security Notes

No persistent state risk: All operations are atomic.
Reentrancy protected: Callback executed in controlled context.
PDA authority: Only program can move NFT from vault.
Fee overflow safe: Uses checked arithmetic.


Use Cases

NFT-backed liquidations
Cross-protocol yield farming
Atomic NFT staking + borrowing
MEV searchers arbitraging NFT floors


License
MIT


See tests/ for full integration examples
See programs/ for instruction logic

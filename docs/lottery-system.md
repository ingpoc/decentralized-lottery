# Lottery System Documentation

## File Structure
1. **Core Program Files**
   ```
   programs/decentralized-lottery/
   ├── src/
   │   ├── lib.rs                 // Program entry and main module definitions
   │   ├── state/                 // Account struct definitions
   │   │   ├── mod.rs            // State module exports
   │   │   ├── global_config.rs  // Global configuration account
   │   │   ├── lottery.rs        // Lottery account structure & enums
   │   │   ├── ticket.rs         // Ticket account structure
   │   │   └── treasury.rs       // (If any specific treasury state, else handled by global_config)
   │   ├── instructions/         // Instruction handlers and their account contexts
   │   │   ├── mod.rs           // Instruction module exports
   │   │   ├── initialize.rs     // Handles program initialization (GlobalConfig)
   │   │   ├── create_lottery.rs // Handles lottery creation
   │   │   ├── buy_ticket.rs    // Handles ticket purchases
   │   │   ├── transition_state.rs // Manages lottery state changes (manual admin transitions)
   │   │   ├── settle_randomness.rs // Handles VRF randomness settlement
   │   │   ├── select_winner.rs  // Handles winner selection using VRF randomness
   │   │   ├── claim_prize.rs   // Handles prize claims by winners
   │   │   └── update_config.rs  // Handles updates to global configuration
   │   ├── errors.rs            // Custom error definitions for the program
   │   ├── events.rs            // Event definitions emitted by instructions
   │   └── utils.rs             // Utility functions (e.g., PDA derivations)
   └── tests/                   // Integration tests (TypeScript)
       └── decentralized-lottery.ts
   ```

## Implementation Patterns

1. **Lottery States**
   ```rust
   pub enum LotteryState {
       Created,            // Initial state after creation, not yet open for ticket sales.
       Open,               // Accepting ticket purchases.
       Locked,             // Ticket sales closed (e.g., draw time reached, before VRF request). Optional manual step.
       Drawing,            // (Largely deprecated in favor of AwaitingRandomness) May represent a brief moment when transitioning, or an admin recovery point.
       AwaitingRandomness, // Randomness requested from VRF, awaiting callback/settlement. Tickets cannot be bought.
       Completed,          // Randomness received and processed. Winner selection can now occur or has occurred.
       Expired,            // Lottery ended without a winner (e.g., no tickets sold, or VRF callback timed out).
       Cancelled           // Lottery manually cancelled by admin. Can happen in various active states.
   }
   ```

2. **Lottery Types**
   ```rust
   pub enum LotteryType {
       Daily,
       Weekly,
       Monthly
   }
   ```

3. **Prize Structure**
   ```rust
   pub struct PrizeTier {
       pub percentage: u8,  // Percentage of total prize pool
       pub winners: u32,    // Number of winners for this tier
   }

   pub struct Winner {
       pub ticket_number: u64,
       pub prize_amount: u64,
       pub tier: u8,
       pub claimed: bool,
       pub winner_address: Pubkey,
   }
   ```

## State Management

1. **Global Configuration**
   ```rust
   pub struct GlobalConfig {
       pub treasury: Pubkey,
       pub treasury_fee_percentage: u16, // Basis points (2.5% = 250)
       pub admin: Pubkey,
       pub usdc_mint: Pubkey,
   }
   ```

   **Configuration Updates**
   - The `update_config` instruction allows updating the USDC mint address without redeployment
   - Only the admin can execute this instruction
   - Updates are atomic and immediately effective for all new lotteries

2. **Lottery Account**
   ```rust
   pub struct LotteryAccount {
       // Core Details
       pub lottery_type: LotteryType,
       pub ticket_price: u64,
       pub draw_time: i64,         // Unix timestamp for when the lottery draw should ideally occur.
       pub authority: Pubkey,      // The authority (creator/admin) for this specific lottery.
       pub global_config: Pubkey,  // Link to the global configuration.
       pub created_by: Pubkey,     // Original creator of the lottery.
       pub created_at: i64,        // Timestamp of creation.

       // State & Prize Management
       pub state: LotteryState,
       pub prize_pool: u64,
       pub total_tickets: u64,
       pub last_ticket_id: u64,    // Counter for generating unique ticket IDs.
       pub winning_ticket: Option<Pubkey>, // PDA of the winning TicketAccount.
       pub completed_at: Option<i64>,  // Timestamp of completion, expiry, or cancellation.
       pub is_prize_pool_locked: bool, // True if prize pool cannot be further modified (e.g., after randomness).
       pub is_claimed: bool,           // True if the main prize has been claimed.
       pub target_prize_pool: u64, // Optional target prize pool amount.

       // VRF (Verifiable Random Function) Fields
       pub vrf_client: Option<Pubkey>,          // Pubkey of the VRF client account (e.g., Switchboard VRF PDA).
       pub vrf_request_key: Option<Pubkey>,     // Unique identifier for the VRF request (often same as vrf_client or derived).
       pub vrf_randomness: Option<[u8; 32]>,  // Stores the raw randomness received from the VRF.
       pub randomness_fulfilled: bool,        // True once valid randomness has been received and stored.
       pub oracle_pubkey: Option<Pubkey>,       // (Currently less used, might be for specific VRF provider details or future use).

       pub auto_transition: bool,  // (Currently not fully implemented) Intended for automatic state changes.
       // Removed: winning_numbers, treasury_fee_percent (now in GlobalConfig), prize_tiers, winners (simplified model), pyth_price_accounts
   }
   ```

## Security Features

1. **Random Number Generation**
   - Utilizes a Verifiable Random Function (VRF) for secure and unpredictable random number generation, crucial for fair winner selection. The current implementation is structured to integrate with a VRF provider like Switchboard.
   - The process involves:
     1. **Request**: When the lottery is ready (e.g., draw time reached, tickets sold), the `transition_state` instruction (moving to `AwaitingRandomness`) makes a placeholder CPI call to the chosen VRF provider to request randomness. The `LotteryAccount` stores a `vrf_request_key` to identify this request.
     2. **Fulfillment**: An off-chain VRF oracle (e.g., operated by Switchboard) detects this request on-chain. It generates a random number along with a cryptographic proof and delivers this back to the VRF provider's on-chain program.
     3. **Settlement**: The `settle_randomness` instruction is then called. This instruction interacts with the VRF provider's program (via CPI) to verify the randomness proof using the stored `vrf_request_key` and retrieves the validated random value.
     4. **Storage**: If valid, the randomness is stored in `LotteryAccount.vrf_randomness`, `randomness_fulfilled` is set to true, and the lottery state transitions to `Completed`.
     5. **Usage**: The `select_winner` instruction uses this verified `vrf_randomness` to determine the winning ticket.

2. **Access Control**
   - Admin-only functions for configuration
   - PDA-based account validation
   - State transition restrictions
   - USDC mint address can only be updated by admin

3. **Fund Management**
   - Treasury fee collection
   - Atomic prize distribution
   - Protected fund transfers

## State Transitions

The lottery progresses through various states, managed by specific instructions. Admin intervention is typically required for manual state changes, while some transitions might be automated by keepers or occur due to user actions (like buying the first ticket if the lottery ATA needs creation).

1.  **Simplified State Flow Diagram**:
    ```
    1. Created --(admin: transition_state to Open)--> Open
    2. Open --(user: buy_ticket)--> Open (tickets sold, prize pool increases)
       (If draw_time passes while Open, buy_ticket blocks; admin must call transition_state)
    3. Open --(admin: transition_state to Drawing, past draw_time, tickets > 0)--> AwaitingRandomness
       (This step initiates VRF request)
    4. Open --(admin: transition_state to Expired, past draw_time, tickets == 0)--> Expired
    5. AwaitingRandomness --(keeper/admin: settle_randomness, after VRF oracle fulfillment)--> Completed
    6. AwaitingRandomness --(admin: transition_state to Expired, if VRF callback times out)--> Expired
    7. Completed --(anyone/admin: select_winner)--> Completed (winning_ticket PDA is set)
    8. Completed --(winner: claim_prize, with correct winning ticket)--> Completed (prize paid out, lottery marked claimed)

    General Transitions:
    - Any active (non-terminal) state --(admin: transition_state to Cancelled)--> Cancelled
    ```

2.  **Key Transition Rules & Logic**:
    *   **Created to Open**: Admin uses `transition_state`. Lottery becomes available for ticket sales.
    *   **Open to AwaitingRandomness**:
        *   Triggered by admin via `transition_state` (targeting `Drawing` state variant) typically after `draw_time` has passed and tickets have been sold.
        *   The `transition_state` instruction initiates the VRF randomness request (placeholder CPI) and sets the state to `AwaitingRandomness`.
        *   If no tickets are sold by `draw_time`, admin can transition to `Expired`.
    *   **AwaitingRandomness to Completed**:
        *   Triggered by `settle_randomness` instruction.
        *   This instruction verifies and stores the VRF randomness.
        *   Sets `randomness_fulfilled = true`.
    *   **AwaitingRandomness to Expired**:
        *   Admin can manually transition to `Expired` via `transition_state` if the VRF callback does not occur within a reasonable timeout period beyond `draw_time`.
    *   **Winner Selection**: The `select_winner` instruction is called when the lottery is `Completed` and `randomness_fulfilled` is true. It uses the stored randomness to pick a winner.
    *   **Cancellation**: Admin can cancel a lottery from most active states using `transition_state`. (Refund logic is typically a separate concern or instruction).

## Events

1. **Lottery Events**
   ```rust
   pub struct LotteryCreated {
       pub lottery_id: Pubkey,
       pub lottery_type: String,
       pub ticket_price: u64,
       pub draw_time: i64,
       pub prize_pool: u64,
   }

   pub struct TicketPurchased {
       pub lottery_id: Pubkey,
       pub buyer: Pubkey,
       pub number_of_tickets: u64,
       pub total_cost: u64,
       pub timestamp: i64,
   }

   pub struct LotteryStateChanged {
       pub lottery_id: Pubkey,
       pub previous_state: LotteryState,
       pub new_state: LotteryState,
       pub timestamp: i64,
       // Optional: additional context like total_tickets_sold, current_prize_pool
   }

   pub struct RandomnessSettled { // Emitted by settle_randomness
       pub lottery_id: Pubkey,
       pub randomness: [u8; 32],
       pub timestamp: i64,
   }

   pub struct LotteryWinnerDetermined { // Emitted by select_winner
       pub lottery_id: Pubkey,
       // previous_state and new_state are likely both 'Completed' here.
       // Consider if these are needed or if other fields are more relevant.
       pub winner: Pubkey,               // PDA of the winning TicketAccount
       pub randomness_source: Pubkey,    // e.g., VRF client account key used for the request
       pub winning_ticket_id: u64,       // Numerical ID of the winning ticket
       pub timestamp: i64,
   }

   pub struct PrizeClaimed { // Emitted by claim_prize
       pub lottery_id: Pubkey,
       pub ticket_id: u64,
       pub winner: Pubkey, // Pubkey of the prize recipient (buyer of the ticket)
       pub prize_pool: u64, // Total prize pool at time of claim
       pub treasury_fee: u64,
       pub winner_payout: u64,
       pub timestamp: i64,
   }
   ```

## Error Handling

1. **Custom Errors**
   ```rust
   pub enum LotteryError {
       InvalidLotteryState,
       UnauthorizedAccess,
       InvalidStateTransition,
       DrawTimeNotReached,
       TicketPurchaseLimitReached,
       // ... other errors
   }
   ```

## Testing Guidelines

1. **Unit Tests**
   ```rust
   #[cfg(test)]
   mod tests {
       #[test]
       fn test_lottery_creation() {
           // Test lottery initialization
       }

       #[test]
       fn test_ticket_purchase() {
           // Test ticket buying logic
       }

       #[test]
       fn test_winner_selection() {
           // Test randomness and selection
       }
   }
   ```

2. **Integration Tests**
   ```typescript
   describe("decentralized-lottery", () => {
       it("Initialize Config and Treasury", async () => {
           // Test initialization
       });

       it("Create Lottery", async () => {
           // Test lottery creation
       });

       it("Buy Ticket", async () => {
           // Test ticket purchase
       });
       // ... other integration tests for state transitions, VRF flow, winner selection, prize claim ...
   });
   ```
   *   **VRF Mocking**: Integration tests for VRF interactions should use a mocked VRF handler or simulate oracle callbacks to test the `settle_randomness` and `select_winner` flow. Unit tests within Rust modules can also mock dependencies.

## Keeper Bots
For fully automated operation (e.g., transitioning lotteries when `draw_time` is reached, or calling `settle_randomness` promptly after VRF fulfillment if not handled by the VRF service itself), an external keeper bot is necessary. This bot would monitor on-chain state and time, and call the appropriate instructions (`transition_state`, `settle_randomness`) when conditions are met.

## Common Pitfalls

1. **Security Issues**
   - Not validating account ownership
   - Incorrect PDA validation
   - Missing access controls
   - Unsafe math operations

2. **State Management**
   - Invalid state transitions
   - Race conditions in ticket purchases
   - Incorrect prize calculations
   - Missing treasury fee collection

3. **Fund Handling**
   - Incorrect token account validation
   - Missing escrow checks
   - Incorrect prize distribution
   - Treasury fee calculation errors

## Implementation Guidelines

1. **Development Workflow**
   ```bash
   # Adding Dependencies
   - ALWAYS use cargo add for new dependencies
   cargo add pyth-sdk-solana --version 0.10.3
   cargo add sha2 --version 0.10.8

   # Building and Testing
   - Clean before major changes
   anchor clean
   
   - Build to verify changes
   anchor build
   
   - Run tests after each feature
   anchor test
   
   # Automated Build Process
   - Full build with IDL and type generation
   npm run build:full
   
   - Update IDL and types only (after anchor build)
   npm run update-idl
   
   # Updating Configuration
   - Update USDC mint address without redeployment
   npm run update-config
   ```

2. **Module Structure Best Practices**
   ```rust
   // lib.rs - Keep the main program structure clean
   use anchor_lang::prelude::*;
   use anchor_spl::token::Mint;

   declare_id!("your_program_id");

   pub mod instructions;
   pub mod state;
   pub mod utils;
   pub mod errors;
   pub mod events;

   #[program]
   pub mod decentralized_lottery {
       use super::*;
       pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
           instructions::initialize::handler(ctx)
       }
       // ... other top-level instruction callers for create_lottery, buy_ticket, etc.
   }

   // instructions/some_instruction.rs - Example of an instruction module
   use anchor_lang::prelude::*;
   // ... other necessary imports like state, errors ...
   #[derive(Accounts)]
   pub struct SomeInstruction<'info> { /* ... accounts ... */ }

   pub fn handler(ctx: Context<SomeInstruction>) -> Result<()> { /* ... logic ... */ }
   ```

3. **Incremental Implementation Order**
   ```
   1. Core Structure
      - Set up basic program structure
      - Implement state definitions
      - Add error handling

   2. Basic Features
      - Initialize
      - Create lottery
      - Buy ticket

   3. Advanced Features
      - Transition state
      - Select winner
      - Claim prize
   ```

4. **Common Issues and Solutions**
   ```
   Issue: Unresolved import `crate`
   Solution: 
   - Keep program macro in lib.rs
   - Use proper module organization
   - Avoid circular dependencies

   Issue: Pyth SDK Integration
   Solution:
   - Use latest SDK version
   - Follow proper import structure
   - Update deprecated functions

   Issue: Build Failures
   Solution:
   - Clean build artifacts
   - Verify dependencies
   - Check module exports
   ```

5. **Import Guidelines**
   ```rust
   // DO:
   use anchor_lang::error_code;  // Specific imports
   use crate::state::lottery::*; // Module imports

   // DON'T:
   use anchor_lang::prelude::*;  // In error.rs if not needed
   use super::*;                 // Outside of program module
   ```

6. **Feature Implementation Checklist**
   ```
   □ Verify dependencies in Cargo.toml
   □ Create necessary module files
   □ Implement state structures
   □ Add error handling
   □ Implement instruction logic
   □ Add event emissions
   □ Write tests
   □ Build and verify
   ```

7. **Testing Strategy**
   ```rust
   // Unit Tests
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_feature() {
           // Test implementation
       }
   }

   // Integration Tests
   describe("Feature", () => {
       before(() => {
           // Setup
       });

       it("should work", async () => {
           // Test
       });
   });
   ```

8. **Dependency Management**
   ```toml
   [dependencies]
   # Core dependencies - DO NOT MODIFY
   anchor-lang = "0.30.1"
   anchor-spl = "0.30.1"
   
   # Feature-specific dependencies
   # Add using: cargo add <package> --version <version>
   pyth-sdk-solana = "0.10.3"
   sha2 = "0.10.8"
   ```

9. **PDA (Program Derived Address) Management**
   ```rust
   // PDA Seeds and Bumps
   - Global Config PDA: [b"global_config"]
   - Lottery Account PDA: [b"lottery", lottery_id.as_ref()]
   - Treasury PDA: [b"treasury", global_config.key().as_ref()]

   // Validation Example
   #[account(
       seeds = [b"lottery", lottery_id.as_ref()],
       bump,
       constraint = lottery_account.state == LotteryState::Open
   )]
   pub lottery_account: Account<'info, LotteryAccount>,
   ```

10. **Instruction Parameter Validation**
    ```rust
    // Required Validations
    - Ticket price > 0
    - Draw time > current time
    - Prize pool >= minimum required
    - Number of tickets within limits
    - Valid lottery state for operation
    - Valid treasury fee percentage (0-1000 basis points)
    ```

11. **State Management Best Practices**
    ```rust
    // State Updates
    - Use atomic updates
    - Verify state before transitions
    - Emit events after state changes
    - Handle edge cases (timeouts, failures)

    // Example
    pub fn transition_state(ctx: Context<TransitionState>) -> Result<()> {
        let lottery = &mut ctx.accounts.lottery_account;
        let current_time = Clock::get()?.unix_timestamp;
        
        require!(
            lottery.state == LotteryState::Open && 
            current_time >= lottery.draw_time,
            LotteryError::InvalidStateTransition
        );

        let previous_state = lottery.state;
        lottery.state = LotteryState::Drawing;

        emit!(LotteryStateChanged {
            lottery_id: lottery.key(),
            previous_state,
            new_state: lottery.state,
            timestamp: current_time,
        });

        Ok(())
    }
    ```

12. **Token Handling Guidelines**
    ```rust
    // Token Operations
    - Always verify token account ownership
    - Check token mint matches expected
    - Use SPL token program for transfers
    - Handle decimal places correctly

    // Example
    #[account(
        mut,
        constraint = ticket_payment.mint == lottery.usdc_mint,
        constraint = ticket_payment.owner == buyer.key(),
    )]
    pub ticket_payment: Account<'info, TokenAccount>,
    ```

13. **Error Handling Strategy**
    ```rust
    // Error Categories
    1. Validation Errors
       - Input validation
       - State validation
       - Account validation

    2. Operation Errors
       - Token operations
       - State transitions
       - Random number generation

    3. System Errors
       - Oracle failures
       - Timeout conditions
       - Resource exhaustion

    // Example
    #[error_code]
    pub enum LotteryError {
        #[msg("Invalid lottery state for operation")]
        InvalidLotteryState,
        #[msg("Token transfer failed")]
        TokenTransferFailed,
        #[msg("Oracle data is stale")]
        StaleOracleData,
    }
    ```

14. **Security Considerations**
    ```rust
    // Security Checklist
    □ Account validation
      □ Owner checks
      □ PDA verification
      □ Signer verification
    
    □ Token security
      □ Mint verification
      □ Balance checks
      □ Transfer authority
    
    □ State protection
      □ Atomic updates
      □ Race condition prevention
      □ Reentrancy guards
    
    □ Access control
      □ Admin operations
      □ User operations
      □ System operations
    ```

15. **Testing Requirements**
    ```rust
    // Test Categories
    1. Unit Tests
       □ State transitions
       □ Input validation
       □ Error conditions
    
    2. Integration Tests
       □ Full lottery lifecycle
       □ Multiple participants
       □ Edge cases
    
    3. Security Tests
       □ Invalid accounts
       □ Unauthorized access
       □ State manipulation
    
    // Example Test Structure
    #[cfg(test)]
    mod tests {
        use super::*;
        
        #[test]
        fn test_lottery_lifecycle() {
            // Setup
            let mut lottery = setup_lottery();
            
            // Create lottery
            assert!(create_lottery(...).is_ok());
            
            // Buy tickets
            assert!(buy_tickets(...).is_ok());
            
            // Select winner
            assert!(select_winner(...).is_ok());
            
            // Verify final state
            assert_eq!(lottery.state, LotteryState::Completed);
        }
    }
    ```

## Configuration Management

1. **USDC Mint Address Updates**
   ```typescript
   // Script to update USDC mint address (update-config.ts)
   import { Connection, PublicKey, Transaction, TransactionInstruction } from '@solana/web3.js';

   // Create the instruction
   const instruction = new TransactionInstruction({
     keys: [
       { pubkey: globalConfigPDA, isSigner: false, isWritable: true },
       { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
       { pubkey: NEW_USDC_MINT, isSigner: false, isWritable: false },
     ],
     programId,
     data,
   });
   ```

2. **Update Process**
   - The update-config script derives the global config PDA
   - Creates a transaction with the update_config instruction
   - Sends the transaction to the Solana network
   - Verifies the update by checking the account data

3. **Verification**
   ```bash
   # Verify transaction on Solana Explorer
   https://explorer.solana.com/tx/{SIGNATURE}?cluster=devnet
   ```

4. **Configuration Tests**
   ```typescript
   describe("Configuration Management", () => {
     it("Should update USDC mint address", async () => {
       // Test update_config instruction
       const newMint = await createMint(provider);
       await program.methods
         .updateConfig()
         .accounts({
           globalConfig: globalConfigPDA,
           admin: provider.wallet.publicKey,
           usdcMint: newMint,
         })
         .rpc();
         
       // Verify the update
       const configAccount = await program.account.globalConfig.fetch(globalConfigPDA);
       assert.equal(configAccount.usdcMint.toString(), newMint.toString());
     });
   });
   ```

## Developer Guidelines for Future Development

1. **USDC Mint Address Considerations**
   - **Frontend Synchronization**: Always ensure frontend code is updated to use the same USDC mint address as the on-chain program
   - **Testing After Updates**: After updating the USDC mint address, test the full lottery lifecycle to ensure token transfers work correctly
   - **Token Account Creation**: Remember that users need token accounts for the specific USDC mint being used
   - **Devnet vs Mainnet**: Use different mint addresses for devnet and mainnet environments
   ```typescript
   // Example of environment-specific configuration
   const USDC_MINT = {
     devnet: new PublicKey("Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr"),
     mainnet: new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
   };
   ```

2. **Program Deployment and Updates**
   - **Program ID Consistency**: When redeploying with the same program ID, no reinitialization is needed
   - **PDA Derivation**: All PDAs remain valid after code updates as long as the program ID stays the same
   - **Account Data Compatibility**: Ensure any changes to account structures are backward compatible
   - **Migration Strategy**: For breaking changes, implement a migration path for existing accounts
   ```rust
   // Example of backward compatible account update
   #[account]
   pub struct GlobalConfig {
       pub treasury: Pubkey,
       pub treasury_fee_percentage: u16,
       pub admin: Pubkey,
       pub usdc_mint: Pubkey,
       // New fields should be added at the end
       pub new_field: Option<u64>, // Make new fields optional for compatibility
   }
   ```

3. **Error Handling for Token Operations**
   - **Token Account Existence**: Check if users have the appropriate token accounts before operations
   - **Balance Verification**: Verify sufficient token balances before attempting transfers
   - **Mint Verification**: Always validate that token accounts match the expected USDC mint
   - **Error Recovery**: Implement proper error handling for failed token transfers
   ```rust
   // Example of robust token account validation
   #[account(
       constraint = ticket_payment.mint == global_config.usdc_mint @ LotteryError::InvalidMint,
       constraint = ticket_payment.owner == buyer.key() @ LotteryError::InvalidOwner,
       constraint = ticket_payment.amount >= ticket_price @ LotteryError::InsufficientFunds,
   )]
   pub ticket_payment: Account<'info, TokenAccount>,
   ```

4. **Frontend Integration Best Practices**
   - **Wallet Connection**: Ensure wallet adapters support the token standard being used
   - **Token Balance Display**: Show users their balance of the specific USDC mint being used
   - **Transaction Monitoring**: Implement proper transaction monitoring and error handling
   - **Configuration Synchronization**: Fetch the current USDC mint from the global config on startup
   ```typescript
   // Example of fetching current configuration
   const fetchCurrentConfig = async () => {
     const [globalConfigPDA] = PublicKey.findProgramAddressSync(
       [Buffer.from('global_config')],
       programId
     );
     
     const configAccount = await program.account.globalConfig.fetch(globalConfigPDA);
     setUsdcMint(configAccount.usdcMint);
   };
   ```

5. **Security Considerations for Updates**
   - **Admin Key Security**: Protect the admin private key used for configuration updates
   - **Multi-Signature**: Consider implementing multi-signature requirements for sensitive operations
   - **Timelock Mechanisms**: Add timelocks for critical configuration changes
   - **Event Logging**: Log all configuration changes for auditability
   ```rust
   // Example of event emission for configuration changes
   emit!(ConfigUpdated {
       previous_mint: old_mint,
       new_mint: new_mint,
       updated_by: admin.key(),
       timestamp: Clock::get()?.unix_timestamp,
   });
   ```

6. **Testing Strategy for Configuration Changes**
   - **Automated Tests**: Create specific tests for configuration update scenarios
   - **Integration Testing**: Test the full lottery lifecycle with the new configuration
   - **Edge Cases**: Test with invalid inputs and unauthorized attempts
   - **Regression Testing**: Ensure existing functionality works with new configuration
   ```typescript
   // Example test cases for configuration updates
   it("Should reject unauthorized update attempts", async () => {
     // Test with non-admin wallet
   });
   
   it("Should maintain existing lotteries after update", async () => {
     // Create lottery, update config, verify lottery still works
   });
   
   it("Should use new mint for new lotteries", async () => {
     // Update config, create new lottery, verify it uses new mint
   });
   ```

7. **Automated Build and IDL Management**
   - **Always Use Automated Scripts**: Use `npm run build:full` for complete builds to ensure IDL and types stay in sync
   - **Frontend Synchronization**: The automated process ensures frontend code uses the latest IDL definitions
   - **Version Control**: Commit both the IDL and generated types to version control for tracking changes
   - **CI/CD Integration**: Include the automated build process in CI/CD pipelines
   ```bash
   # Complete build process
   npm run build:full  # Cleans, builds, and updates all IDL files and types
   
   # After making changes to the program
   npm run build       # Builds and updates IDL files without cleaning
   
   # After manual anchor build
   npm run update-idl  # Updates IDL files and types only
   ``` 
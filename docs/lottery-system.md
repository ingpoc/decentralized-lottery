# Lottery System Documentation

## File Structure
1. **Core Files**
   ```
   programs/decentralized-lottery/
   ├── src/
   │   ├── lib.rs                 // Program entry and instruction handlers
   │   ├── state/                 // State definitions
   │   │   ├── mod.rs            // State module exports
   │   │   ├── lottery.rs        // Lottery account structures
   │   │   └── treasury.rs       // Treasury and config structures
   │   ├── instructions/         // Instruction implementations
   │   │   ├── mod.rs           // Instruction module exports
   │   │   ├── create_lottery.rs // Lottery creation
   │   │   ├── buy_ticket.rs    // Ticket purchase
   │   │   ├── transition_state.rs // State management
   │   │   ├── select_winner.rs  // Winner selection
   │   │   ├── claim_prize.rs   // Prize distribution
   │   │   └── update_config.rs  // Configuration updates
   │   ├── errors.rs            // Custom error definitions
   │   ├── events.rs            // Event definitions
   │   └── utils.rs             // Utility functions
   └── tests/                   // Integration tests
   ```

## Implementation Patterns

1. **Lottery States**
   ```rust
   pub enum LotteryState {
       Created,    // Initial state after creation
       Open,       // Accepting ticket purchases
       Drawing,    // Winner selection in progress
       Completed,  // Winners selected
       Expired     // Past claim deadline
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
       pub lottery_type: LotteryType,
       pub ticket_price: u64,
       pub draw_time: i64,
       pub prize_pool: u64,
       pub total_tickets: u64,
       pub winning_numbers: Option<Vec<u8>>,
       pub state: LotteryState,
       pub created_by: Pubkey,
       pub global_config: Pubkey,
       pub treasury_fee_percent: u8,
       pub prize_tiers: Vec<PrizeTier>,
       pub winners: Vec<Winner>,
       pub last_ticket_id: u64,
       pub pyth_price_accounts: Vec<Pubkey>,
       pub auto_transition: bool,
   }
   ```

## Security Features

1. **Random Number Generation**
   - Uses Pyth price feeds for randomness source
   - Combines multiple price feeds with timestamps
   - SHA256 hashing for final number generation

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

1. **Valid Transitions**
   ```
   Created -> Open -> Drawing -> Completed
                              -> Expired
   ```

2. **Transition Rules**
   - Created to Open: Before draw time
   - Open to Drawing: At draw time
   - Drawing to Completed: After winner selection
   - Drawing to Expired: If selection fails/times out

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
   });
   ```

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
       // Instruction handlers
   }
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
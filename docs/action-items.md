Pending Action Items:

1. Lottery Drawing Mechanism: While there are functions to create lotteries and buy tickets, there      doesn't appear to be an implementation for the actual lottery drawing mechanism. This would include:
    - Random number generation for winner selection
    - Prize distribution to winners
    - Handling multiple winners scenario

2. Ticket Refund System: There's no implementation for handling refunds if a lottery is cancelled or if certain conditions aren't met.

3. Admin Controls: While there's an admin role defined in the initialization, there are no admin-specific functions implemented for:
    - Pausing/resuming lottery operations
    - Updating lottery parameters
    - Emergency functions for critical situations

4. Treasury Management: While there's a treasury fee percentage set in initialization (2.5%), there doesn't seem to be implementation for:
    - Treasury fee collection
    - Fee withdrawal mechanism
    - Treasury balance management

5. User Features:
    - Viewing ticket history
    - Checking winning status
    - Claiming prizes
    - Viewing active lotteries

6. Documentation:
    - Add documentation to the codebase
    - Update README with usage instructions
    - Create a detailed documentation file (docs/lottery-system.md)

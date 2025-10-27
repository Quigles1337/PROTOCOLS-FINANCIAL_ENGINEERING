module xrpl_primitives::account_delete {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::table::{Self, Table};
    use sui::coin::{Self, Coin};
    use sui::sui::SUI;
    use sui::balance::{Self, Balance};
    use sui::event;

    // Errors
    const ERR_ACCOUNT_EXISTS: u64 = 1;
    const ERR_ACCOUNT_NOT_FOUND: u64 = 2;
    const ERR_NOT_AUTHORIZED: u64 = 3;
    const ERR_ACCOUNT_NOT_ACTIVE: u64 = 4;
    const ERR_NO_DELETION_PENDING: u64 = 5;
    const ERR_GRACE_PERIOD_NOT_ELAPSED: u64 = 6;
    const ERR_SELF_BENEFICIARY: u64 = 7;
    const ERR_INVALID_DEPOSIT: u64 = 8;

    // Status
    const STATUS_ACTIVE: u8 = 0;
    const STATUS_PENDING_DELETION: u8 = 1;
    const STATUS_DELETED: u8 = 2;

    // Grace period: 24 hours in epochs (assuming ~24 second epochs)
    const GRACE_PERIOD: u64 = 3600;

    // Structs
    public struct Account has store {
        owner: address,
        balance: Balance<SUI>,
        status: u8,
        deletion_request_time: u64,
        beneficiary: Option<address>,
        created_at: u64,
    }

    public struct AccountRegistry has key {
        id: UID,
        accounts: Table<address, Account>,
    }

    // Events
    public struct AccountCreated has copy, drop {
        owner: address,
    }

    public struct DeletionRequested has copy, drop {
        owner: address,
        beneficiary: address,
    }

    public struct DeletionCancelled has copy, drop {
        owner: address,
    }

    public struct AccountDeleted has copy, drop {
        owner: address,
        beneficiary: address,
        amount: u64,
    }

    // Initialize registry
    fun init(ctx: &mut TxContext) {
        let registry = AccountRegistry {
            id: object::new(ctx),
            accounts: table::new(ctx),
        };
        transfer::share_object(registry);
    }

    // Create account
    public entry fun create_account(
        registry: &mut AccountRegistry,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(!table::contains(&registry.accounts, owner), ERR_ACCOUNT_EXISTS);

        let account = Account {
            owner,
            balance: balance::zero(),
            status: STATUS_ACTIVE,
            deletion_request_time: 0,
            beneficiary: option::none(),
            created_at: tx_context::epoch(ctx),
        };

        table::add(&mut registry.accounts, owner, account);

        event::emit(AccountCreated {
            owner,
        });
    }

    // Deposit funds
    public entry fun deposit(
        registry: &mut AccountRegistry,
        deposit: Coin<SUI>,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(table::contains(&registry.accounts, owner), ERR_ACCOUNT_NOT_FOUND);

        let account = table::borrow_mut(&mut registry.accounts, owner);
        assert!(account.status == STATUS_ACTIVE, ERR_ACCOUNT_NOT_ACTIVE);

        let deposit_value = coin::value(&deposit);
        assert!(deposit_value > 0, ERR_INVALID_DEPOSIT);

        balance::join(&mut account.balance, coin::into_balance(deposit));
    }

    // Request deletion
    public entry fun request_deletion(
        registry: &mut AccountRegistry,
        beneficiary: address,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(owner != beneficiary, ERR_SELF_BENEFICIARY);
        assert!(table::contains(&registry.accounts, owner), ERR_ACCOUNT_NOT_FOUND);

        let account = table::borrow_mut(&mut registry.accounts, owner);
        assert!(account.status == STATUS_ACTIVE, ERR_ACCOUNT_NOT_ACTIVE);

        account.status = STATUS_PENDING_DELETION;
        account.deletion_request_time = tx_context::epoch(ctx);
        account.beneficiary = option::some(beneficiary);

        event::emit(DeletionRequested {
            owner,
            beneficiary,
        });
    }

    // Cancel deletion
    public entry fun cancel_deletion(
        registry: &mut AccountRegistry,
        ctx: &mut TxContext
    ) {
        let owner = tx_context::sender(ctx);
        assert!(table::contains(&registry.accounts, owner), ERR_ACCOUNT_NOT_FOUND);

        let account = table::borrow_mut(&mut registry.accounts, owner);
        assert!(account.status == STATUS_PENDING_DELETION, ERR_NO_DELETION_PENDING);

        account.status = STATUS_ACTIVE;
        account.deletion_request_time = 0;
        account.beneficiary = option::none();

        event::emit(DeletionCancelled {
            owner,
        });
    }

    // Execute deletion
    public entry fun execute_deletion(
        registry: &mut AccountRegistry,
        account_id: address,
        ctx: &mut TxContext
    ) {
        assert!(table::contains(&registry.accounts, account_id), ERR_ACCOUNT_NOT_FOUND);

        let account = table::borrow_mut(&mut registry.accounts, account_id);
        assert!(account.status == STATUS_PENDING_DELETION, ERR_NO_DELETION_PENDING);

        let elapsed = tx_context::epoch(ctx) - account.deletion_request_time;
        assert!(elapsed >= GRACE_PERIOD, ERR_GRACE_PERIOD_NOT_ELAPSED);

        let beneficiary = *option::borrow(&account.beneficiary);
        let balance_value = balance::value(&account.balance);

        account.status = STATUS_DELETED;

        if (balance_value > 0) {
            let transfer_balance = balance::withdraw_all(&mut account.balance);
            let transfer_coin = coin::from_balance(transfer_balance, ctx);
            transfer::public_transfer(transfer_coin, beneficiary);
        };

        event::emit(AccountDeleted {
            owner: account_id,
            beneficiary,
            amount: balance_value,
        });
    }

    // View functions
    public fun can_delete(registry: &AccountRegistry, account_id: address, current_epoch: u64): bool {
        if (table::contains(&registry.accounts, account_id)) {
            let account = table::borrow(&registry.accounts, account_id);
            if (account.status == STATUS_PENDING_DELETION) {
                let elapsed = current_epoch - account.deletion_request_time;
                elapsed >= GRACE_PERIOD
            } else {
                false
            }
        } else {
            false
        }
    }

    public fun get_time_until_deletion(
        registry: &AccountRegistry,
        account_id: address,
        current_epoch: u64
    ): u64 {
        if (table::contains(&registry.accounts, account_id)) {
            let account = table::borrow(&registry.accounts, account_id);
            if (account.status == STATUS_PENDING_DELETION) {
                let elapsed = current_epoch - account.deletion_request_time;
                if (elapsed >= GRACE_PERIOD) {
                    0
                } else {
                    GRACE_PERIOD - elapsed
                }
            } else {
                0
            }
        } else {
            0
        }
    }
}

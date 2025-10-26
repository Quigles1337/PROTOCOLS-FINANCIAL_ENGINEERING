module xrpl_primitives::account_delete {
    use std::signer;
    use std::error;
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_std::table::{Self, Table};

    /// Errors
    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_REQUEST_NOT_FOUND: u64 = 3;
    const E_UNAUTHORIZED: u64 = 4;
    const E_GRACE_PERIOD_NOT_ENDED: u64 = 5;
    const E_ALREADY_EXECUTED: u64 = 6;
    const E_REQUEST_EXISTS: u64 = 7;
    const E_ACCOUNT_NOT_FOUND: u64 = 8;

    /// Account lifecycle status
    const STATUS_ACTIVE: u8 = 1;
    const STATUS_PENDING_DELETION: u8 = 2;
    const STATUS_DELETED: u8 = 3;

    /// Grace period (24 hours in seconds)
    const GRACE_PERIOD: u64 = 86400;

    /// Account information
    struct AccountInfo has store, copy, drop {
        owner: address,
        status: u8,
        created_at: u64,
    }

    /// Deletion request
    struct DeletionRequest has store {
        owner: address,
        beneficiary: address,
        grace_period_end: u64,
        executed: bool,
        created_at: u64,
    }

    /// Global registry
    struct AccountRegistry has key {
        accounts: Table<address, AccountInfo>,
        deletion_requests: Table<address, DeletionRequest>,
        account_balances: Table<address, Coin<AptosCoin>>,
    }

    /// Events
    #[event]
    struct AccountCreated has drop, store {
        owner: address,
        timestamp: u64,
    }

    #[event]
    struct DeletionRequested has drop, store {
        owner: address,
        beneficiary: address,
        grace_period_end: u64,
        timestamp: u64,
    }

    #[event]
    struct DeletionCancelled has drop, store {
        owner: address,
        timestamp: u64,
    }

    #[event]
    struct AccountDeleted has drop, store {
        owner: address,
        beneficiary: address,
        balance_transferred: u64,
        timestamp: u64,
    }

    /// Initialize the registry
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<AccountRegistry>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, AccountRegistry {
            accounts: table::new(),
            deletion_requests: table::new(),
            account_balances: table::new(),
        });
    }

    /// Create an account
    public entry fun create_account(
        owner: &signer,
    ) acquires AccountRegistry {
        let owner_addr = signer::address_of(owner);

        let registry_addr = @xrpl_primitives;
        assert!(exists<AccountRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<AccountRegistry>(registry_addr);

        let account_info = AccountInfo {
            owner: owner_addr,
            status: STATUS_ACTIVE,
            created_at: timestamp::now_seconds(),
        };

        table::add(&mut registry.accounts, owner_addr, account_info);

        event::emit(AccountCreated {
            owner: owner_addr,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Deposit funds to account
    public entry fun deposit(
        depositor: &signer,
        account: address,
        amount: u64,
    ) acquires AccountRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<AccountRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<AccountRegistry>(registry_addr);
        assert!(table::contains(&registry.accounts, account), error::not_found(E_ACCOUNT_NOT_FOUND));

        let coins = coin::withdraw<AptosCoin>(depositor, amount);

        if (table::contains(&registry.account_balances, account)) {
            let existing_balance = table::borrow_mut(&mut registry.account_balances, account);
            coin::merge(existing_balance, coins);
        } else {
            table::add(&mut registry.account_balances, account, coins);
        };
    }

    /// Request account deletion
    public entry fun request_deletion(
        owner: &signer,
        beneficiary: address,
    ) acquires AccountRegistry {
        let owner_addr = signer::address_of(owner);

        let registry_addr = @xrpl_primitives;
        assert!(exists<AccountRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<AccountRegistry>(registry_addr);
        assert!(table::contains(&registry.accounts, owner_addr), error::not_found(E_ACCOUNT_NOT_FOUND));
        assert!(!table::contains(&registry.deletion_requests, owner_addr), error::already_exists(E_REQUEST_EXISTS));

        let now = timestamp::now_seconds();
        let grace_period_end = now + GRACE_PERIOD;

        let deletion_request = DeletionRequest {
            owner: owner_addr,
            beneficiary,
            grace_period_end,
            executed: false,
            created_at: now,
        };

        table::add(&mut registry.deletion_requests, owner_addr, deletion_request);

        let account_info = table::borrow_mut(&mut registry.accounts, owner_addr);
        account_info.status = STATUS_PENDING_DELETION;

        event::emit(DeletionRequested {
            owner: owner_addr,
            beneficiary,
            grace_period_end,
            timestamp: now,
        });
    }

    /// Cancel deletion request
    public entry fun cancel_deletion(
        owner: &signer,
    ) acquires AccountRegistry {
        let owner_addr = signer::address_of(owner);

        let registry_addr = @xrpl_primitives;
        assert!(exists<AccountRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<AccountRegistry>(registry_addr);
        assert!(table::contains(&registry.deletion_requests, owner_addr), error::not_found(E_REQUEST_NOT_FOUND));

        let deletion_request = table::borrow(&registry.deletion_requests, owner_addr);
        assert!(deletion_request.owner == owner_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(!deletion_request.executed, error::invalid_state(E_ALREADY_EXECUTED));

        table::remove(&mut registry.deletion_requests, owner_addr);

        let account_info = table::borrow_mut(&mut registry.accounts, owner_addr);
        account_info.status = STATUS_ACTIVE;

        event::emit(DeletionCancelled {
            owner: owner_addr,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Execute account deletion (after grace period)
    public entry fun execute_deletion(
        owner_or_beneficiary: &signer,
        account_to_delete: address,
    ) acquires AccountRegistry {
        let caller = signer::address_of(owner_or_beneficiary);

        let registry_addr = @xrpl_primitives;
        assert!(exists<AccountRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<AccountRegistry>(registry_addr);
        assert!(table::contains(&registry.deletion_requests, account_to_delete), error::not_found(E_REQUEST_NOT_FOUND));

        let deletion_request = table::borrow_mut(&mut registry.deletion_requests, account_to_delete);
        assert!(
            caller == deletion_request.owner || caller == deletion_request.beneficiary,
            error::permission_denied(E_UNAUTHORIZED)
        );
        assert!(!deletion_request.executed, error::invalid_state(E_ALREADY_EXECUTED));

        let now = timestamp::now_seconds();
        assert!(now >= deletion_request.grace_period_end, error::invalid_state(E_GRACE_PERIOD_NOT_ENDED));

        // Transfer any remaining balance to beneficiary
        let balance_transferred = 0u64;
        if (table::contains(&registry.account_balances, account_to_delete)) {
            let balance = table::remove(&mut registry.account_balances, account_to_delete);
            balance_transferred = coin::value(&balance);
            if (balance_transferred > 0) {
                coin::deposit(deletion_request.beneficiary, balance);
            } else {
                coin::destroy_zero(balance);
            };
        };

        deletion_request.executed = true;

        let account_info = table::borrow_mut(&mut registry.accounts, account_to_delete);
        account_info.status = STATUS_DELETED;

        event::emit(AccountDeleted {
            owner: account_to_delete,
            beneficiary: deletion_request.beneficiary,
            balance_transferred,
            timestamp: now,
        });
    }

    /// View functions
    #[view]
    public fun get_account(owner: address): (u8, u64) acquires AccountRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<AccountRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<AccountRegistry>(registry_addr);
        assert!(table::contains(&registry.accounts, owner), error::not_found(E_ACCOUNT_NOT_FOUND));

        let account_info = table::borrow(&registry.accounts, owner);
        (account_info.status, account_info.created_at)
    }

    #[view]
    public fun get_deletion_request(owner: address): (address, u64, bool) acquires AccountRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<AccountRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<AccountRegistry>(registry_addr);
        assert!(table::contains(&registry.deletion_requests, owner), error::not_found(E_REQUEST_NOT_FOUND));

        let deletion_request = table::borrow(&registry.deletion_requests, owner);
        (deletion_request.beneficiary, deletion_request.grace_period_end, deletion_request.executed)
    }

    #[view]
    public fun get_balance(owner: address): u64 acquires AccountRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<AccountRegistry>(registry_addr)) {
            return 0
        };

        let registry = borrow_global<AccountRegistry>(registry_addr);
        if (!table::contains(&registry.account_balances, owner)) {
            return 0
        };

        let balance = table::borrow(&registry.account_balances, owner);
        coin::value(balance)
    }

    #[view]
    public fun is_pending_deletion(owner: address): bool acquires AccountRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<AccountRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<AccountRegistry>(registry_addr);
        if (!table::contains(&registry.accounts, owner)) {
            return false
        };

        let account_info = table::borrow(&registry.accounts, owner);
        account_info.status == STATUS_PENDING_DELETION
    }

    #[view]
    public fun can_execute_deletion(owner: address): bool acquires AccountRegistry {
        let registry_addr = @xrpl_primitives;
        if (!exists<AccountRegistry>(registry_addr)) {
            return false
        };

        let registry = borrow_global<AccountRegistry>(registry_addr);
        if (!table::contains(&registry.deletion_requests, owner)) {
            return false
        };

        let deletion_request = table::borrow(&registry.deletion_requests, owner);
        if (deletion_request.executed) {
            return false
        };

        let now = timestamp::now_seconds();
        now >= deletion_request.grace_period_end
    }
}

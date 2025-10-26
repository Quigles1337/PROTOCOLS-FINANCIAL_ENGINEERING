module xrpl_primitives::payment_channels {
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
    const E_CHANNEL_NOT_FOUND: u64 = 3;
    const E_CHANNEL_CLOSED: u64 = 4;
    const E_UNAUTHORIZED: u64 = 5;
    const E_INVALID_AMOUNT: u64 = 6;
    const E_INSUFFICIENT_FUNDS: u64 = 7;
    const E_EXPIRATION_NOT_REACHED: u64 = 8;
    const E_CLAIM_TOO_HIGH: u64 = 9;
    const E_SAME_ACCOUNT: u64 = 10;

    /// Payment channel status
    const STATUS_OPEN: u8 = 1;
    const STATUS_CLOSED: u8 = 2;
    const STATUS_EXPIRED: u8 = 3;

    /// PaymentChannel for streaming micropayments
    struct PaymentChannel has store {
        sender: address,
        receiver: address,
        balance: u64,
        total_claimed: u64,
        expiration: u64,
        status: u8,
        created_at: u64,
    }

    /// Global channel registry
    struct ChannelRegistry has key {
        channels: Table<u64, PaymentChannel>,
        deposits: Table<u64, Coin<AptosCoin>>,
        next_id: u64,
    }

    /// Events
    #[event]
    struct ChannelCreated has drop, store {
        channel_id: u64,
        sender: address,
        receiver: address,
        balance: u64,
        expiration: u64,
        timestamp: u64,
    }

    #[event]
    struct FundsAdded has drop, store {
        channel_id: u64,
        amount: u64,
        new_balance: u64,
        timestamp: u64,
    }

    #[event]
    struct FundsClaimed has drop, store {
        channel_id: u64,
        receiver: address,
        amount: u64,
        total_claimed: u64,
        timestamp: u64,
    }

    #[event]
    struct ChannelClosed has drop, store {
        channel_id: u64,
        sender: address,
        receiver: address,
        final_balance: u64,
        timestamp: u64,
    }

    /// Initialize the channel registry
    public entry fun initialize(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<ChannelRegistry>(addr), error::already_exists(E_ALREADY_INITIALIZED));

        move_to(account, ChannelRegistry {
            channels: table::new(),
            deposits: table::new(),
            next_id: 0,
        });
    }

    /// Create a new payment channel
    public entry fun create_channel(
        sender: &signer,
        receiver: address,
        initial_balance: u64,
        expiration: u64,
    ) acquires ChannelRegistry {
        let sender_addr = signer::address_of(sender);
        assert!(sender_addr != receiver, error::invalid_argument(E_SAME_ACCOUNT));
        assert!(initial_balance > 0, error::invalid_argument(E_INVALID_AMOUNT));

        let registry_addr = @xrpl_primitives;
        assert!(exists<ChannelRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let now = timestamp::now_seconds();
        assert!(expiration > now, error::invalid_argument(E_EXPIRATION_NOT_REACHED));

        let registry = borrow_global_mut<ChannelRegistry>(registry_addr);
        let channel_id = registry.next_id;
        registry.next_id = channel_id + 1;

        // Withdraw coins from sender
        let coins = coin::withdraw<AptosCoin>(sender, initial_balance);

        let channel = PaymentChannel {
            sender: sender_addr,
            receiver,
            balance: initial_balance,
            total_claimed: 0,
            expiration,
            status: STATUS_OPEN,
            created_at: now,
        };

        table::add(&mut registry.channels, channel_id, channel);
        table::add(&mut registry.deposits, channel_id, coins);

        event::emit(ChannelCreated {
            channel_id,
            sender: sender_addr,
            receiver,
            balance: initial_balance,
            expiration,
            timestamp: now,
        });
    }

    /// Add funds to existing channel
    public entry fun add_funds(
        sender: &signer,
        channel_id: u64,
        amount: u64,
    ) acquires ChannelRegistry {
        let sender_addr = signer::address_of(sender);
        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));

        let registry_addr = @xrpl_primitives;
        assert!(exists<ChannelRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<ChannelRegistry>(registry_addr);
        assert!(table::contains(&registry.channels, channel_id), error::not_found(E_CHANNEL_NOT_FOUND));

        let channel = table::borrow_mut(&mut registry.channels, channel_id);
        assert!(channel.sender == sender_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(channel.status == STATUS_OPEN, error::invalid_state(E_CHANNEL_CLOSED));

        // Withdraw and merge coins
        let new_coins = coin::withdraw<AptosCoin>(sender, amount);
        let existing_coins = table::borrow_mut(&mut registry.deposits, channel_id);
        coin::merge(existing_coins, new_coins);

        channel.balance = channel.balance + amount;

        event::emit(FundsAdded {
            channel_id,
            amount,
            new_balance: channel.balance,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Claim funds from channel (receiver only)
    public entry fun claim_funds(
        receiver: &signer,
        channel_id: u64,
        amount: u64,
    ) acquires ChannelRegistry {
        let receiver_addr = signer::address_of(receiver);
        assert!(amount > 0, error::invalid_argument(E_INVALID_AMOUNT));

        let registry_addr = @xrpl_primitives;
        assert!(exists<ChannelRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<ChannelRegistry>(registry_addr);
        assert!(table::contains(&registry.channels, channel_id), error::not_found(E_CHANNEL_NOT_FOUND));

        let channel = table::borrow_mut(&mut registry.channels, channel_id);
        assert!(channel.receiver == receiver_addr, error::permission_denied(E_UNAUTHORIZED));
        assert!(channel.status == STATUS_OPEN, error::invalid_state(E_CHANNEL_CLOSED));

        let available = channel.balance - channel.total_claimed;
        assert!(amount <= available, error::invalid_argument(E_CLAIM_TOO_HIGH));

        // Extract coins and deposit to receiver
        let deposit = table::borrow_mut(&mut registry.deposits, channel_id);
        let claimed_coins = coin::extract(deposit, amount);
        coin::deposit(receiver_addr, claimed_coins);

        channel.total_claimed = channel.total_claimed + amount;

        event::emit(FundsClaimed {
            channel_id,
            receiver: receiver_addr,
            amount,
            total_claimed: channel.total_claimed,
            timestamp: timestamp::now_seconds(),
        });
    }

    /// Close channel (sender can close after expiration, receiver can close anytime)
    public entry fun close_channel(
        account: &signer,
        channel_id: u64,
    ) acquires ChannelRegistry {
        let addr = signer::address_of(account);

        let registry_addr = @xrpl_primitives;
        assert!(exists<ChannelRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global_mut<ChannelRegistry>(registry_addr);
        assert!(table::contains(&registry.channels, channel_id), error::not_found(E_CHANNEL_NOT_FOUND));

        let channel = table::borrow_mut(&mut registry.channels, channel_id);
        assert!(channel.status == STATUS_OPEN, error::invalid_state(E_CHANNEL_CLOSED));

        let now = timestamp::now_seconds();

        // Authorization check
        if (channel.sender == addr) {
            // Sender can only close after expiration
            assert!(now >= channel.expiration, error::permission_denied(E_EXPIRATION_NOT_REACHED));
        } else if (channel.receiver == addr) {
            // Receiver can close anytime
        } else {
            // Unauthorized
            abort error::permission_denied(E_UNAUTHORIZED)
        };

        // Return unclaimed funds to sender
        let remaining = channel.balance - channel.total_claimed;
        if (remaining > 0) {
            let deposit = table::borrow_mut(&mut registry.deposits, channel_id);
            let return_coins = coin::extract(deposit, remaining);
            coin::deposit(channel.sender, return_coins);
        };

        channel.status = STATUS_CLOSED;

        event::emit(ChannelClosed {
            channel_id,
            sender: channel.sender,
            receiver: channel.receiver,
            final_balance: channel.total_claimed,
            timestamp: now,
        });
    }

    /// View functions
    #[view]
    public fun get_channel(channel_id: u64): (address, address, u64, u64, u64, u8) acquires ChannelRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<ChannelRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<ChannelRegistry>(registry_addr);
        assert!(table::contains(&registry.channels, channel_id), error::not_found(E_CHANNEL_NOT_FOUND));

        let channel = table::borrow(&registry.channels, channel_id);
        (channel.sender, channel.receiver, channel.balance, channel.total_claimed, channel.expiration, channel.status)
    }

    #[view]
    public fun get_available_balance(channel_id: u64): u64 acquires ChannelRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<ChannelRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<ChannelRegistry>(registry_addr);
        assert!(table::contains(&registry.channels, channel_id), error::not_found(E_CHANNEL_NOT_FOUND));

        let channel = table::borrow(&registry.channels, channel_id);
        channel.balance - channel.total_claimed
    }

    #[view]
    public fun is_expired(channel_id: u64): bool acquires ChannelRegistry {
        let registry_addr = @xrpl_primitives;
        assert!(exists<ChannelRegistry>(registry_addr), error::not_found(E_NOT_INITIALIZED));

        let registry = borrow_global<ChannelRegistry>(registry_addr);
        assert!(table::contains(&registry.channels, channel_id), error::not_found(E_CHANNEL_NOT_FOUND));

        let channel = table::borrow(&registry.channels, channel_id);
        timestamp::now_seconds() >= channel.expiration
    }
}

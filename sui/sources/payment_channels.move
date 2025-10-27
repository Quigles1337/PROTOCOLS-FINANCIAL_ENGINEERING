module xrpl_primitives::payment_channels {
    use sui::object::{Self, UID};
    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::coin::{Self, Coin};
    use sui::sui::SUI;
    use sui::balance::{Self, Balance};
    use sui::event;

    // Errors
    const ERR_NOT_AUTHORIZED: u64 = 1;
    const ERR_CHANNEL_EXPIRED: u64 = 2;
    const ERR_INSUFFICIENT_FUNDS: u64 = 3;
    const ERR_INVALID_AMOUNT: u64 = 4;
    const ERR_CHANNEL_NOT_OPEN: u64 = 5;

    // Channel status
    const STATUS_OPEN: u8 = 0;
    const STATUS_CLOSED: u8 = 1;

    // Structs
    public struct Channel has key {
        id: UID,
        sender: address,
        receiver: address,
        balance: Balance<SUI>,
        total_claimed: u64,
        expiration: u64,
        status: u8,
    }

    // Events
    public struct ChannelCreated has copy, drop {
        channel_id: address,
        sender: address,
        receiver: address,
        deposit: u64,
        expiration: u64,
    }

    public struct FundsClaimed has copy, drop {
        channel_id: address,
        amount: u64,
    }

    public struct ChannelClosed has copy, drop {
        channel_id: address,
    }

    // Create channel
    public entry fun create_channel(
        deposit: Coin<SUI>,
        receiver: address,
        expiration: u64,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        let deposit_value = coin::value(&deposit);
        assert!(deposit_value > 0, ERR_INVALID_AMOUNT);
        assert!(expiration > tx_context::epoch(ctx), ERR_CHANNEL_EXPIRED);

        let channel = Channel {
            id: object::new(ctx),
            sender,
            receiver,
            balance: coin::into_balance(deposit),
            total_claimed: 0,
            expiration,
            status: STATUS_OPEN,
        };

        let channel_id = object::uid_to_address(&channel.id);

        event::emit(ChannelCreated {
            channel_id,
            sender,
            receiver,
            deposit: deposit_value,
            expiration,
        });

        transfer::share_object(channel);
    }

    // Add funds to channel
    public entry fun add_funds(
        channel: &mut Channel,
        deposit: Coin<SUI>,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(sender == channel.sender, ERR_NOT_AUTHORIZED);
        assert!(channel.status == STATUS_OPEN, ERR_CHANNEL_NOT_OPEN);

        let deposit_value = coin::value(&deposit);
        assert!(deposit_value > 0, ERR_INVALID_AMOUNT);

        balance::join(&mut channel.balance, coin::into_balance(deposit));
    }

    // Claim funds
    public entry fun claim_funds(
        channel: &mut Channel,
        amount: u64,
        ctx: &mut TxContext
    ) {
        let receiver = tx_context::sender(ctx);
        assert!(receiver == channel.receiver, ERR_NOT_AUTHORIZED);
        assert!(channel.status == STATUS_OPEN, ERR_CHANNEL_NOT_OPEN);
        assert!(tx_context::epoch(ctx) < channel.expiration, ERR_CHANNEL_EXPIRED);

        let total_balance = balance::value(&channel.balance);
        let available = total_balance + channel.total_claimed;
        let claimable = available - channel.total_claimed;
        assert!(amount <= claimable, ERR_INSUFFICIENT_FUNDS);
        assert!(amount <= total_balance, ERR_INSUFFICIENT_FUNDS);

        channel.total_claimed = channel.total_claimed + amount;

        let claimed_balance = balance::split(&mut channel.balance, amount);
        let claimed_coin = coin::from_balance(claimed_balance, ctx);

        event::emit(FundsClaimed {
            channel_id: object::uid_to_address(&channel.id),
            amount,
        });

        transfer::public_transfer(claimed_coin, receiver);
    }

    // Close channel
    public entry fun close_channel(
        channel: &mut Channel,
        ctx: &mut TxContext
    ) {
        let sender = tx_context::sender(ctx);
        assert!(sender == channel.sender || sender == channel.receiver, ERR_NOT_AUTHORIZED);
        assert!(channel.status == STATUS_OPEN, ERR_CHANNEL_NOT_OPEN);

        channel.status = STATUS_CLOSED;

        let remaining = balance::value(&channel.balance);
        if (remaining > 0) {
            let refund_balance = balance::withdraw_all(&mut channel.balance);
            let refund_coin = coin::from_balance(refund_balance, ctx);
            transfer::public_transfer(refund_coin, channel.sender);
        };

        event::emit(ChannelClosed {
            channel_id: object::uid_to_address(&channel.id),
        });
    }

    // View functions
    public fun get_balance(channel: &Channel): u64 {
        balance::value(&channel.balance)
    }

    public fun get_available_balance(channel: &Channel): u64 {
        let total_balance = balance::value(&channel.balance);
        total_balance + channel.total_claimed - channel.total_claimed
    }
}

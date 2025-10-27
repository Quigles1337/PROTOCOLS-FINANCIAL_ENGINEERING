use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault, Promise};
use serde::{Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Order {
    pub creator: AccountId,
    pub sell_asset: String,
    pub buy_asset: String,
    pub sell_amount: Balance,
    pub buy_amount: Balance,
    pub filled_amount: Balance,
    pub status: OrderStatus,
    pub created_at: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct DEXContract {
    orders: UnorderedMap<u64, Order>,
    next_id: u64,
}

#[near_bindgen]
impl DEXContract {
    #[init]
    pub fn new() -> Self {
        Self {
            orders: UnorderedMap::new(b"o"),
            next_id: 0,
        }
    }

    #[payable]
    pub fn place_order(
        &mut self,
        sell_asset: String,
        buy_asset: String,
        sell_amount: Balance,
        buy_amount: Balance,
    ) -> u64 {
        let creator = env::predecessor_account_id();
        let deposit = env::attached_deposit();

        assert!(sell_amount > 0 && buy_amount > 0, "Invalid amounts");
        assert_ne!(sell_asset, buy_asset, "Assets must differ");

        // For NEAR native token orders, require deposit
        if sell_asset == "NEAR" {
            assert_eq!(deposit, sell_amount, "Deposit must match sell amount");
        }

        let order_id = self.next_id;
        self.next_id += 1;

        let order = Order {
            creator,
            sell_asset,
            buy_asset,
            sell_amount,
            buy_amount,
            filled_amount: 0,
            status: OrderStatus::Open,
            created_at: env::block_timestamp(),
        };

        self.orders.insert(&order_id, &order);
        order_id
    }

    #[payable]
    pub fn fill_order(&mut self, order_id: u64, fill_amount: Balance) -> Promise {
        let filler = env::predecessor_account_id();
        let deposit = env::attached_deposit();

        let mut order = self.orders.get(&order_id).expect("Order not found");
        assert!(
            order.status == OrderStatus::Open || order.status == OrderStatus::PartiallyFilled,
            "Order not available"
        );

        let remaining = order.sell_amount - order.filled_amount;
        assert!(fill_amount > 0 && fill_amount <= remaining, "Invalid fill amount");

        // Calculate proportional payment
        let payment = (fill_amount * order.buy_amount) / order.sell_amount;

        // For NEAR native token, verify deposit
        if order.buy_asset == "NEAR" {
            assert_eq!(deposit, payment, "Deposit must match payment");
        }

        order.filled_amount += fill_amount;

        if order.filled_amount == order.sell_amount {
            order.status = OrderStatus::Filled;
        } else {
            order.status = OrderStatus::PartiallyFilled;
        }

        self.orders.insert(&order_id, &order);

        // Transfer filled amount to filler
        // In production, this would integrate with token contracts
        if order.sell_asset == "NEAR" {
            Promise::new(filler).transfer(fill_amount)
        } else {
            // For non-NEAR assets, would call token contract
            Promise::new(filler).transfer(0)
        }
    }

    pub fn cancel_order(&mut self, order_id: u64) -> Promise {
        let creator = env::predecessor_account_id();

        let mut order = self.orders.get(&order_id).expect("Order not found");
        assert_eq!(order.creator, creator, "Not authorized");
        assert!(
            order.status == OrderStatus::Open || order.status == OrderStatus::PartiallyFilled,
            "Cannot cancel"
        );

        let remaining = order.sell_amount - order.filled_amount;

        order.status = OrderStatus::Cancelled;
        self.orders.insert(&order_id, &order);

        // Return remaining deposit for NEAR orders
        if order.sell_asset == "NEAR" {
            Promise::new(creator).transfer(remaining)
        } else {
            Promise::new(creator).transfer(0)
        }
    }

    pub fn get_order(&self, order_id: u64) -> Option<Order> {
        self.orders.get(&order_id)
    }

    pub fn get_remaining_amount(&self, order_id: u64) -> Balance {
        if let Some(order) = self.orders.get(&order_id) {
            order.sell_amount - order.filled_amount
        } else {
            0
        }
    }
}

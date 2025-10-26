#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod dex_orders {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct DEXOrders {
        admin: AccountId,
        orders: Mapping<u64, Order>,
        order_counter: u64,
        orderbook: Mapping<(AssetId, AssetId), Vec<u64>>,
    }

    pub type AssetId = u32;

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct Order {
        pub id: u64,
        pub maker: AccountId,
        pub sell_asset: AssetId,
        pub buy_asset: AssetId,
        pub sell_amount: Balance,
        pub buy_amount: Balance,
        pub filled: Balance,
        pub status: OrderStatus,
        pub created_at: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub enum OrderStatus {
        Open,
        PartiallyFilled,
        Filled,
        Cancelled,
    }

    #[ink(event)]
    pub struct OrderPlaced {
        #[ink(topic)]
        order_id: u64,
        maker: AccountId,
        sell_asset: AssetId,
        buy_asset: AssetId,
        sell_amount: Balance,
        buy_amount: Balance,
    }

    #[ink(event)]
    pub struct OrderFilled {
        #[ink(topic)]
        order_id: u64,
        taker: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct OrderCancelled {
        #[ink(topic)]
        order_id: u64,
    }

    impl DEXOrders {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                orders: Mapping::new(),
                order_counter: 0,
                orderbook: Mapping::new(),
            }
        }

        #[ink(message, payable)]
        pub fn place_order(
            &mut self,
            sell_asset: AssetId,
            buy_asset: AssetId,
            sell_amount: Balance,
            buy_amount: Balance,
        ) -> u64 {
            let maker = self.env().caller();
            let deposit = self.env().transferred_value();
            let current_block = self.env().block_number();

            assert!(sell_amount > 0, "Sell amount must be positive");
            assert!(buy_amount > 0, "Buy amount must be positive");
            assert!(sell_asset != buy_asset, "Assets must be different");
            assert!(deposit >= sell_amount, "Insufficient deposit");

            self.order_counter += 1;
            let order_id = self.order_counter;

            let order = Order {
                id: order_id,
                maker,
                sell_asset,
                buy_asset,
                sell_amount,
                buy_amount,
                filled: 0,
                status: OrderStatus::Open,
                created_at: current_block,
            };

            self.orders.insert(order_id, &order);

            self.env().emit_event(OrderPlaced {
                order_id,
                maker,
                sell_asset,
                buy_asset,
                sell_amount,
                buy_amount,
            });

            order_id
        }

        #[ink(message, payable)]
        pub fn fill_order(&mut self, order_id: u64, fill_amount: Balance) {
            let taker = self.env().caller();
            let payment = self.env().transferred_value();
            let mut order = self.orders.get(order_id).expect("Order not found");

            assert!(
                matches!(order.status, OrderStatus::Open | OrderStatus::PartiallyFilled),
                "Order not available"
            );
            assert!(fill_amount > 0, "Fill amount must be positive");

            let remaining = order.sell_amount - order.filled;
            let actual_fill = if fill_amount > remaining {
                remaining
            } else {
                fill_amount
            };

            let required_payment = (actual_fill * order.buy_amount) / order.sell_amount;
            assert!(payment >= required_payment, "Insufficient payment");

            order.filled += actual_fill;

            if order.filled >= order.sell_amount {
                order.status = OrderStatus::Filled;
            } else {
                order.status = OrderStatus::PartiallyFilled;
            }

            self.orders.insert(order_id, &order);

            self.env()
                .transfer(taker, actual_fill)
                .expect("Transfer to taker failed");

            self.env()
                .transfer(order.maker, required_payment)
                .expect("Transfer to maker failed");

            if payment > required_payment {
                let refund = payment - required_payment;
                self.env()
                    .transfer(taker, refund)
                    .expect("Refund failed");
            }

            self.env().emit_event(OrderFilled {
                order_id,
                taker,
                amount: actual_fill,
            });
        }

        #[ink(message)]
        pub fn cancel_order(&mut self, order_id: u64) {
            let caller = self.env().caller();
            let mut order = self.orders.get(order_id).expect("Order not found");

            assert!(caller == order.maker, "Only maker can cancel");
            assert!(
                matches!(order.status, OrderStatus::Open | OrderStatus::PartiallyFilled),
                "Order not cancellable"
            );

            let refund = order.sell_amount - order.filled;
            order.status = OrderStatus::Cancelled;
            self.orders.insert(order_id, &order);

            if refund > 0 {
                self.env()
                    .transfer(order.maker, refund)
                    .expect("Refund failed");
            }

            self.env().emit_event(OrderCancelled { order_id });
        }

        #[ink(message)]
        pub fn get_order(&self, order_id: u64) -> Option<Order> {
            self.orders.get(order_id)
        }

        #[ink(message)]
        pub fn get_price(&self, order_id: u64) -> Option<(Balance, Balance)> {
            self.orders.get(order_id).map(|o| (o.buy_amount, o.sell_amount))
        }

        #[ink(message)]
        pub fn get_order_count(&self) -> u64 {
            self.order_counter
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn test_place_order() {
            let mut contract = DEXOrders::new();
            let order_id = contract.place_order(1, 2, 100, 200);
            assert_eq!(order_id, 1);
        }
    }
}

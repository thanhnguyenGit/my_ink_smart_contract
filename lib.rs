#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod file_store {
    use core::usize;

    use ink::env::transferred_value;
    use ink::prelude::format;
    use ink::prelude::vec::Vec;
    use ink::storage::{Lazy, Mapping, StorageVec};
    use scale::{Decode, Encode};

    #[ink(storage)]
    pub struct FileStore {
        orders: Vec<(u32, Order)>,
        orders_mapping: Mapping<u32, Order>,
    }

    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Order {
        list_of_items: Vec<FoodItem>,
        customer: AccountId,
        total_price: Balance,
        paid: bool,
        order_id: u32,
    }

    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct FoodItem {
        burger_menu: BurgerMenu,
        amount: u32,
    }

    #[derive(Encode, Decode, Debug, Clone)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum BurgerMenu {
        CheeseBurger,
        ChickenBurger,
        VeggieBurger,
    }

    #[ink::event]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink::event]
    pub struct GetAllOrders {
        #[ink(topic)]
        orders: Vec<(u32, Order)>,
    }

    #[ink::event]
    pub struct GetSingleOrder {
        #[ink(topic)]
        single_order: Order,
    }

    #[ink::event]
    pub struct CreatedShopAndStorage {
        #[ink(topic)]
        orders: Vec<(u32, Order)>,
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum BurgerShopError {
        PaymentError,
        OrderNotCompleted,
    }

    pub type ContractResult<T> = core::result::Result<T, BurgerShopError>;

    impl FileStore {
        #[ink(constructor)]
        pub fn new() -> Self {
            let order_storage_vector: Vec<(u32, Order)> = Vec::new();
            let order_storage_mapping = Mapping::new();

            Self {
                orders: order_storage_vector,
                orders_mapping: order_storage_mapping,
            }
        }

        #[ink(message, payable)]
        pub fn take_order_and_payment(
            &mut self,
            list_of_items: Vec<FoodItem>,
        ) -> ContractResult<Order> {
            let caller = self.env().caller();
            assert!(
                caller != self.env().account_id(),
                "You are not the customer!"
            );
            for item in &list_of_items {
                assert!(item.amount > 0, "Can't take an empty order");
            }
            let id = self.orders.len() as u32;
            let total_price = Order::total_price(&list_of_items);
            let mut order = Order::new(list_of_items, caller, id);
            order.total_price = total_price;
            assert!(
                order.paid == false,
                "Can't pay for an order that is already been paid"
            );
            let multiply: Balance = 1_000_000_000_000;
            let transferred_val = self.env().transferred_value();
            assert!(
                transferred_val
                    == order
                        .total_price
                        .checked_mul(multiply)
                        .expect("Overflow!!!"),
                "{}",
                format!("Please pay complete amount which is {}", order.total_price)
            );
            ink::env::debug_println!("Expected value: {}", order.total_price);
            ink::env::debug_println!(
                "Expected received payment without conversion: {}",
                transferred_val
            );
            match self
                .env()
                .transfer(self.env().account_id(), order.total_price)
            {
                Ok(_) => {
                    let id = self.orders.len() as u32;
                    order.paid = true;
                    self.env().emit_event(Transfer {
                        from: Some(order.customer),
                        to: Some(self.env().account_id()),
                        value: order.total_price,
                    });

                    self.orders_mapping.insert(id, &order);
                    self.orders.push((id, order.clone()));
                    Ok(order)
                }
                Err(_) => Err(BurgerShopError::PaymentError),
            }
        }

        #[ink(message)]
        pub fn get_single_order(&self, id: u32) -> Order {
            let single_order = self.orders_mapping.get(id).expect("Order not found");
            single_order
        }

        #[ink(message)]
        pub fn get_orders(&self) -> Option<Vec<(u32, Order)>> {
            let get_all_orders = &self.orders;

            if get_all_orders.len() > 0 {
                Some(get_all_orders.to_vec())
            } else {
                None
            }
        }
    }

    impl Order {
        fn new(list_of_items: Vec<FoodItem>, customer: AccountId, id: u32) -> Self {
            let total_price = Order::total_price(&list_of_items);
            Self {
                list_of_items,
                customer,
                total_price,
                paid: false,
                order_id: id,
            }
        }

        fn total_price(list_of_items: &[FoodItem]) -> Balance {
            let mut total: u128 = 0;
            for item in list_of_items.iter() {
                total = total.wrapping_add(item.price())
            }
            total
        }
    }

    impl FoodItem {
        fn price(&self) -> Balance {
            match self.burger_menu {
                BurgerMenu::CheeseBurger => BurgerMenu::CheeseBurger
                    .price()
                    .wrapping_mul(self.amount as u128),
                BurgerMenu::ChickenBurger => BurgerMenu::ChickenBurger
                    .price()
                    .wrapping_mul(self.amount as u128),
                BurgerMenu::VeggieBurger => BurgerMenu::VeggieBurger
                    .price()
                    .wrapping_mul(self.amount as u128),
            }
        }
    }

    impl BurgerMenu {
        fn price(&self) -> Balance {
            match self {
                Self::CheeseBurger => 12,
                Self::VeggieBurger => 10,
                Self::ChickenBurger => 15,
            }
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            //let file_store = FileStore::default();
            //assert_eq!(file_store.get(), false);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn it_works() {
            //let mut file_store = FileStore::new(false);
            //assert_eq!(file_store.get(), false);
            //file_store.flip();
            //assert_eq!(file_store.get(), true);
        }
    }

    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::ContractsBackend;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let mut constructor = FileStoreRef::default();

            // When
            let contract = client
                .instantiate("file_store", &ink_e2e::alice(), &mut constructor)
                .submit()
                .await
                .expect("instantiate failed");
            let call_builder = contract.call_builder::<FileStore>();

            // Then
            let get = call_builder.get();
            let get_result = client.call(&ink_e2e::alice(), &get).dry_run().await?;
            assert!(matches!(get_result.return_value(), false));

            Ok(())
        }

        /// We test that we can read and write a value from the on-chain contract.
        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let mut constructor = FileStoreRef::new(false);
            let contract = client
                .instantiate("file_store", &ink_e2e::bob(), &mut constructor)
                .submit()
                .await
                .expect("instantiate failed");
            let mut call_builder = contract.call_builder::<FileStore>();

            let get = call_builder.get();
            let get_result = client.call(&ink_e2e::bob(), &get).dry_run().await?;
            assert!(matches!(get_result.return_value(), false));

            // When
            let flip = call_builder.flip();
            let _flip_result = client
                .call(&ink_e2e::bob(), &flip)
                .submit()
                .await
                .expect("flip failed");

            // Then
            let get = call_builder.get();
            let get_result = client.call(&ink_e2e::bob(), &get).dry_run().await?;
            assert!(matches!(get_result.return_value(), true));

            Ok(())
        }
    }
}

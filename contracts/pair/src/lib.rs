#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod pair {
    use ink::env::{
        call::{build_call, ExecutionInput, Selector},
        DefaultEnvironment,
    };
    use ink::prelude::vec::Vec;
    use scale::CompactAs;
    use sp_arithmetic::FixedU128;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Overflow,
        TokenMintingFailed,
    }

    #[ink(event)]
    pub struct Sync {
        reserve_0: u128,
        reserve_1: u128,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(event)]
    pub struct Mint {
        #[ink(topic)]
        owner: Option<AccountId>,
        amount_0: Balance,
        amount_1: Balance,
    }

    #[ink(event)]
    pub struct Burn {
        #[ink(topic)]
        sender: Option<AccountId>,
        amount_0: Balance,
        amount_1: Balance,
        to: AccountId,
    }

    #[ink(event)]
    pub struct Swap {
        sender: Option<AccountId>,
        amount_0_in: Balance,
        amount_1_in: Balance,
        amount_0_out: Balance,
        amount_1_out: Balance,
        to: AccountId,
    }

    const MINIMUM_LIQUIDITY: u64 = 10_u64.pow(3_u32);
    // Defines the storage of your contract.
    // Add new fields to the below struct in order
    // to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Pair {
        /// Stores the factory address
        factory: AccountId,
        /// Token defined as Token A
        token_0: AccountId,
        /// Token defined as Token B
        token_1: AccountId,
        reserve_0: u128,
        reserve_1: u128,
        price_0_cumulative_last: u128,
        price_1_cumulative_last: u128,
        k_last: u128,
        psp22token: AccountId,
        total_supply: Balance,
        block_timestamp_last: u128,
        // unlocked: u32,
    }

    impl Pair {
        #[ink(constructor)]
        pub fn new(
            factory: AccountId,
            psp22token: AccountId,
            token_0: AccountId,
            token_1: AccountId,
        ) -> Self {
            Self {
                factory,
                token_0,
                token_1,
                psp22token,
                reserve_0: 0,
                reserve_1: 0,
                price_0_cumulative_last: 0,
                price_1_cumulative_last: 0,
                k_last: 0,
                total_supply: 0,
                block_timestamp_last: 0,
                // unlocked: 1
            }
        }

        #[ink(constructor)]
        pub fn initialize(psp22: AccountId, token_0: AccountId, token_1: AccountId) -> Self {
            let caller = Self::env().caller();
            Self::new(caller, psp22, token_0, token_1)
        }

        #[ink(message)]
        pub fn get_reserves(&self) -> (u128, u128, u128) {
            let reserve_0 = self.reserve_0;
            let reserve_1 = self.reserve_1;
            let timestamp = self.block_timestamp_last;
            (reserve_0, reserve_1, timestamp)
        }

        #[ink(message)]
        pub fn update(
            &mut self,
            balance_0: u128,
            balance_1: u128,
            reserve_0: u128,
            reserve_1: u128,
        ) -> Result<()> {
            if balance_0 <= u128::MAX && balance_1 <= u128::MAX {
                let block_timestamp = (self.env().block_timestamp() % 2_u64.pow(32)) as u32;
                let time_elapsed = block_timestamp - self.block_timestamp_last as u32;

                if time_elapsed > 0 && reserve_0 != 0 && reserve_1 != 0 {
                    self.price_0_cumulative_last += reserve_0
                        .checked_div(reserve_1)
                        .unwrap()
                        .checked_mul(time_elapsed as u128)
                        .unwrap();
                    self.price_1_cumulative_last += reserve_1
                        .checked_div(reserve_0)
                        .unwrap()
                        .checked_mul(time_elapsed as u128)
                        .unwrap();
                }

                self.reserve_0 = balance_0;
                self.reserve_1 = balance_1;
                self.block_timestamp_last = block_timestamp as u128;

                self.env().emit_event(Sync {
                    reserve_0: self.reserve_0,
                    reserve_1: self.reserve_1,
                });

                Ok(())
            } else {
                return Err(Error::Overflow);
            }
        }

        #[ink(message)]
        pub fn mint(&mut self) {
            let (reserve_0, reserve_1, _) = self.get_reserves();
            let balance_0 = self.get_token_balance(self.psp22token, self.token_0);
            let balance_1 = self.get_token_balance(self.psp22token, self.token_1);

            let amount_0 = balance_0 - reserve_0;
            let amount_1 = balance_1 - reserve_1;

            let fee_on = self.get_fee_to() != AccountId::from([0x0; 32]);
            let liquidity = if self.get_total_supply() == 0 {
                let mint_result = build_call::<DefaultEnvironment>()
                    .call(self.psp22token)
                    .gas_limit(0)
                    .exec_input(
                        ExecutionInput::new(Selector::new(ink::selector_bytes!("mint")))
                            .push_arg(&self.get_fee_to())
                            .push_arg(&MINIMUM_LIQUIDITY),
                    )
                    .returns::<()>()
                    .try_invoke();

                match mint_result {
                    Ok(Ok(_)) => Ok(()),
                    _ => Err(Error::TokenMintingFailed),
                };
                self.get_squareroot(amount_1.checked_mul(amount_1).unwrap()) as u64
                    - MINIMUM_LIQUIDITY
            } else {
                let liquidity_0 =
                    amount_0.checked_mul(self.get_total_supply()).unwrap() / reserve_0;
                let liquidity_1 =
                    amount_1.checked_mul(self.get_total_supply()).unwrap() / reserve_1;
                if liquidity_0 > liquidity_1 {
                    liquidity_0 as u64
                } else {
                    liquidity_1 as u64
                }
            };

            assert!(liquidity > 0, "UniswapV2: INSUFFICIENT_LIQUIDITY_MINTED");
            let mint_result = build_call::<DefaultEnvironment>()
                .call(self.psp22token)
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("mint")))
                        .push_arg(&self.get_fee_to())
                        .push_arg(&liquidity),
                )
                .returns::<()>()
                .try_invoke();

            match mint_result {
                Ok(Ok(_)) => Ok(()),
                _ => Err(Error::TokenMintingFailed),
            };

            self.update(balance_0, balance_1, reserve_0, reserve_1);
            self.k_last = if fee_on {
                reserve_0.checked_mul(reserve_1).unwrap()
            } else {
                self.k_last
            };
            self.env().emit_event(Mint {
                owner: Some(self.env().caller()),
                amount_0,
                amount_1,
            });
        }

        #[ink(message)]
        pub fn burn(&mut self, to: AccountId) {
            let (reserve_0, reserve_1, _) = self.get_reserves();
            let balance_0 = self.get_token_balance(self.token_0, self.token_0);
            let balance_1 = self.get_token_balance(self.token_1, self.token_1);

            let liquidity = self.get_token_balance(self.psp22token, self.env().account_id());

            let fee_to = self.get_fee_to();
            let amount_0 = liquidity.checked_mul(balance_0).unwrap();
            let amount_1 = liquidity.checked_mul(balance_1).unwrap();

            assert!(
                amount_0 > 0 && amount_1 > 0,
                "UNISWAPV2: INSUFFICIENT_LIQUIDITY_BURNED"
            );

            build_call::<DefaultEnvironment>()
                .call(self.psp22token)
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("burn")))
                        .push_arg(&self.env().account_id())
                        .push_arg(&liquidity),
                )
                .returns::<()>()
                .try_invoke();

            self.transfer_from(self.token_0, self.env().account_id(), to, amount_0);
            self.transfer_from(self.token_1, self.env().account_id(), to, amount_1);

            let balance_0 = self.get_token_balance(self.token_0, self.env().account_id());
            let balance_1 = self.get_token_balance(self.token_1, self.env().account_id());

            self.update(balance_0, balance_1, reserve_0, reserve_1);
            self.k_last = if fee_to != AccountId::from([0x0; 32]) {
                reserve_0.checked_mul(reserve_1).unwrap()
            } else {
                self.k_last
            };
            self.env().emit_event(Burn {
                sender: Some(self.env().caller()),
                amount_0,
                amount_1,
                to,
            });
        }

        #[ink(message)]
        pub fn swap(
            &mut self,
            amount_0_out: Balance,
            amount_1_out: Balance,
            to: AccountId,
            data: Vec<u8>,
        ) {
            assert!(
                amount_0_out > 0 || amount_1_out > 0,
                "UNISWAPV2: INSUFFICIENT_OUTPUT_AMOUNT"
            );
            let (reserve_0, reserve_1, _) = self.get_reserves();
            assert!(
                amount_0_out < reserve_0 && amount_1_out < reserve_1,
                "UNISWAPV2: INSUFFICIENT_LIQUIDITY"
            );

            let mut balance_0: Balance;
            let mut balance_1: Balance;
            {
                assert!(
                    to != self.token_0 && to != self.token_1,
                    "UniswapV2: INVALID_TO_ADDRESS"
                );
                if amount_0_out > 0 {
                    self.transfer_from(self.token_0, self.env().account_id(), to, amount_0_out);
                }
                if amount_1_out > 0 {
                    self.transfer_from(self.token_1, self.env().account_id(), to, amount_1_out);
                }
                balance_0 = self.get_token_balance(self.token_0, self.env().account_id());
                balance_1 = self.get_token_balance(self.token_1, self.env().account_id());
            }

            let amount_0_in: u128 = if balance_0 > (reserve_0 - amount_0_out) {
                balance_0 - (reserve_0 - amount_0_out)
            } else {
                0
            };
            let amount_1_in: u128 = if balance_1 > (reserve_1 - amount_1_out) {
                balance_1 - (reserve_1 - amount_1_out)
            } else {
                0
            };
            assert!(
                amount_0_in > 0 || amount_1_in > 0,
                "Uniswap: INSUFFICIENT_INPUT_AMOUNT"
            );
            {
                let balance_0_adjusted = (balance_0 as u128)
                    .checked_mul(1000)
                    .unwrap()
                    .checked_sub((amount_0_in.checked_mul(3)).unwrap())
                    .unwrap();
                let balance_1_adjusted =
                    balance_1.checked_mul(1000).unwrap() - (amount_1_in.checked_mul(3).unwrap());
                // require(balance0Adjusted.mul(balance1Adjusted) >= uint(_reserve0).mul(_reserve1).mul(1000**2), 'UniswapV2: K');
                assert!(
                    balance_0_adjusted.checked_mul(balance_1_adjusted).unwrap()
                        >= reserve_0
                            .checked_mul(reserve_1)
                            .unwrap()
                            .checked_mul(1000_u128.checked_pow(2).unwrap())
                            .unwrap(),
                    "UniswapV2: K"
                )
            }

            self.update(balance_0, balance_1, reserve_0, reserve_1);
            self.env().emit_event(Swap {
                sender: Some(self.env().caller()),
                amount_0_in,
                amount_1_in,
                amount_0_out,
                amount_1_out,
                to,
            });
        }

        #[ink(message)]
        pub fn skim(&mut self, to: AccountId) {
            let balance_0_of =
                self.get_token_balance(self.token_0, self.env().account_id()) - self.reserve_0;
            let balance_1_of =
                self.get_token_balance(self.token_1, self.env().account_id()) - self.reserve_1;

            self.transfer_from(self.token_0, self.env().account_id(), to, balance_0_of);
            self.transfer_from(self.token_1, self.env().account_id(), to, balance_1_of);
        }

        #[ink(message)]
        pub fn sync(&mut self) {
            let balance_0 = self.get_token_balance(self.token_0, self.env().account_id());
            let balance_1 = self.get_token_balance(self.token_1, self.env().account_id());
            self.update(balance_0, balance_1, self.reserve_0, self.reserve_1);
        }

        #[ink(message)]
        pub fn get_token_balance(&self, token: AccountId, owner: AccountId) -> Balance {
            let result = build_call::<DefaultEnvironment>()
                .call(token)
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("mint")))
                        .push_arg(&owner),
                )
                .returns::<Balance>()
                .try_invoke();

            match result {
                Ok(Ok(value)) => value,
                _ => 0,
            }
        }

        #[ink(message)]
        pub fn get_total_supply(&self) -> Balance {
            let result = build_call::<DefaultEnvironment>()
                .call(self.psp22token)
                .gas_limit(0)
                .exec_input(ExecutionInput::new(Selector::new(ink::selector_bytes!(
                    "total_supply"
                ))))
                .returns::<Balance>()
                .try_invoke();

            match result {
                Ok(Ok(value)) => value,
                _ => 0,
            }
        }

        #[ink(message)]
        pub fn transfer_from(
            &self,
            token: AccountId,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) {
            build_call::<DefaultEnvironment>()
                .call(token)
                .gas_limit(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("transfer_from")))
                        .push_arg(&from)
                        .push_arg(&to)
                        .push_arg(&value),
                )
                .returns::<()>()
                .try_invoke();
        }

        #[ink(message)]
        pub fn get_fee_to(&self) -> AccountId {
            let fee_to_result = build_call::<DefaultEnvironment>()
                .call(self.factory)
                .gas_limit(0)
                .exec_input(ExecutionInput::new(Selector::new(ink::selector_bytes!(
                    "get_fee_to"
                ))))
                .returns::<AccountId>()
                .try_invoke();

            let fee_to = match fee_to_result {
                Ok(Ok(account)) => account,
                _ => AccountId::from([0x0; 32]),
            };

            fee_to
        }

        pub fn mint_fee(&mut self, reserve_0: u128, reserve_1: u128) -> Result<()> {
            let fee_to_result = build_call::<DefaultEnvironment>()
                .call(self.factory)
                .gas_limit(0)
                .exec_input(ExecutionInput::new(Selector::new(ink::selector_bytes!(
                    "get_fee_to"
                ))))
                .returns::<AccountId>()
                .try_invoke();

            let fee_to = match fee_to_result {
                Ok(Ok(account)) => account,
                _ => AccountId::from([0x0; 32]),
            };

            if fee_to != AccountId::from([0x0; 32]) {
                if self.k_last != 0 {
                    let root_k = self
                        .get_squareroot(reserve_0)
                        .checked_mul(reserve_1)
                        .unwrap();
                    let root_k_last = self.get_squareroot(reserve_1);

                    if root_k > root_k_last {
                        let numerator =
                            self.total_supply.checked_mul(root_k - root_k_last).unwrap();
                        let denominator = root_k
                            .checked_mul(5)
                            .unwrap()
                            .checked_add(root_k_last)
                            .unwrap();
                        let liquidity = numerator / denominator;

                        if liquidity > 0 {
                            let mint_result = build_call::<DefaultEnvironment>()
                                .call(self.psp22token)
                                .gas_limit(0)
                                .exec_input(
                                    ExecutionInput::new(Selector::new(ink::selector_bytes!(
                                        "mint"
                                    )))
                                    .push_arg(&fee_to)
                                    .push_arg(&liquidity),
                                )
                                .returns::<()>()
                                .try_invoke();

                            match mint_result {
                                Ok(Ok(_)) => Ok(()),
                                _ => Err(Error::TokenMintingFailed),
                            };
                        }
                    }
                }
            } else {
                self.k_last = 0;
            }

            Ok(())
        }

        pub fn get_squareroot(&self, num: u128) -> u128 {
            let d1 = FixedU128::from_u32(num as u32);
            let d2 = FixedU128::sqrt(d1);
            let d3 = *d2.encode_as();
            d3
        }
    }
}

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod core {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    // (address indexed token0, address indexed token1, address pair, uint);
    #[ink(event)]
    pub struct PairCreated {
        #[ink(topic)]
        token0: Option<AccountId>,
        #[ink(topic)]
        token1: Option<AccountId>,
        pair: Option<AccountId>,
        pair_index: u32,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct UniswapCore {
        /// Stores a single `bool` value on the storage.
        fee_to: AccountId,
        fee_to_setter: AccountId,
        get_pairs: Mapping<(AccountId, AccountId), AccountId>,
        all_pairs: Vec<AccountId>,
    }

    impl UniswapCore {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(fee_to_setter: AccountId) -> Self {
            Self {
                fee_to_setter,
                fee_to: AccountId::from([0x0; 32]),
                get_pairs: Mapping::new(),
                all_pairs: Vec::new(),
            }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(AccountId::from([0x0; 32]))
        }
        // function createPair(address tokenA, address tokenB) external returns (address pair) {
        //     require(getPair[token0][token1] == address(0), 'UniswapV2: PAIR_EXISTS'); // single check is sufficient
        //     bytes memory bytecode = type(UniswapV2Pair).creationCode;
        //     bytes32 salt = keccak256(abi.encodePacked(token0, token1));
        //     assembly {
        //         pair := create2(0, add(bytecode, 32), mload(bytecode), salt)
        //     }
        //     IUniswapV2Pair(pair).initialize(token0, token1);
        //     getPair[token0][token1] = pair;
        //     getPair[token1][token0] = pair; // populate mapping in the reverse direction
        //     allPairs.push(pair);
        //     emit PairCreated(token0, token1, pair, allPairs.length);
        // }

        #[ink(message)]
        pub fn create_pair(&mut self, token_a: AccountId, token_b: AccountId) -> Result<(), ()> {
            assert!(token_a != token_b, "Uniswap: IDENTICAL_ADDRESSES");
            assert!(token_a != Self::zero_address(), "Uniswap: ZERO_ADDRESS");
            match self.get_pairs.get((token_a, token_b)) {
                Some(_) => panic!("Uniswap: PAIR_EXISTS"),
                None => {}
            }

            Ok(())
        }

        #[ink(message)]
        pub fn all_pairs_length(&self) -> u32 {
            self.all_pairs.len() as u32
        }

        #[ink(message)]
        pub fn set_fee_to(&mut self, fee_to: AccountId) {
            let caller = self.env().caller();
            assert!(caller == self.fee_to_setter, "Uniswap: Forbidden Caller");

            self.fee_to = fee_to;
        }

        #[ink(message)]
        pub fn set_fee_to_setter(&mut self, fee_to_setter: AccountId) {
            let caller = self.env().caller();
            assert!(caller == self.fee_to_setter, "Uniswap: Forbidden Caller");
            self.fee_to_setter = fee_to_setter;
        }

        pub fn zero_address() -> AccountId {
            AccountId::from([0x0; 32])
        }
    }
}

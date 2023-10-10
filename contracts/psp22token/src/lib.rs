#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(PSP22, PSP22Burnable, PSP22Mintable, PSP22Metadata)]
#[openbrush::contract]
pub mod psp22token {
    use openbrush::traits::Storage;

    #[ink(event)]
    pub struct TransferEvent {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct ApprovalEvent {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance,
    }

    #[ink(storage)]
    #[derive(Storage, Default)]
    pub struct Psp22token {
        #[storage_field]
        psp22: psp22::Data,
        #[storage_field]
        metadata: metadata::Data,
    }

    #[ink::trait_definition]
    pub trait BasePSP22token {}

    impl Psp22token {
        #[ink(constructor)]
        pub fn new(
            total_supply: Balance,
            name: Option<String>,
            symbol: Option<String>,
            decimal: u8,
        ) -> Self {
            let mut instance = Self::default();
            <dyn psp22::Internal>::_mint_to(&mut instance, Self::env().caller(), total_supply)
                .expect("Shoud mint");
            instance.metadata.name.set(&name);
            instance.metadata.symbol.set(&symbol);
            instance.metadata.decimals.set(&decimal);
            instance
        }
    }
}

#![no_std]

use multiversx_sc::types::BigUint;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
struct Order<M: ManagedTypeApi> {
    token_in: TokenIdentifier<M>,
    amount_in: BigUint<M>,
    token_out: TokenIdentifier<M>,
    amount_out_min: BigUint<M>,
    owner: ManagedAddress<M>,
    deadline: u64,
    executed: bool,
    cancelled: bool,
}

mod swap_tokens_proxy {
    multiversx_sc::imports!();

    pub type SwapTokensFixedInputResultType<BigUint> = EsdtTokenPayment<BigUint>;

    #[multiversx_sc::proxy]
    pub trait SwapTokensInterface {
        #[payable("*")]
        #[endpoint(swapTokensFixedInput)]
        fn swap_tokens_fixed_input(
            &self,
            token_out: TokenIdentifier,
            amount_out_min: BigUint
        ) -> SwapTokensFixedInputResultType<Self::Api>;
    }
}

#[multiversx_sc::contract]
pub trait OrderBookContract {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint]
    fn actual_swap_fixed_input(
        &self,
        pair_address: ManagedAddress,
        token_out: TokenIdentifier,
        amount_out_min: BigUint
    ) -> EsdtTokenPayment<Self::Api> {
        require!(amount_out_min > 0, "Amount out min must be greater than 0");

        let (token_in, amount_in) = self.call_value().single_fungible_esdt();

        self.pair_contract_proxy(pair_address)
            .swap_tokens_fixed_input(token_out, amount_out_min)
            .with_esdt_transfer(EsdtTokenPayment::new(token_in, 0, amount_in))
            .async_call()
            .call_and_exit();
        }

    #[payable("*")]
    #[endpoint]
    fn open_order(
        &self,
        token_in: TokenIdentifier,
        amount_in: BigUint,
        token_out: TokenIdentifier,
        amount_out_min: BigUint
    ) {

    }

    #[proxy]
    fn pair_contract_proxy(&self, pair_address: ManagedAddress) -> swap_tokens_proxy::Proxy<Self::Api>;
}

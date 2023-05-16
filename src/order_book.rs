#![no_std]

multiversx_sc::imports!();

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

#![no_std]

use multiversx_sc::types::BigUint;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct Order<M: ManagedTypeApi> {
    pub owner: ManagedAddress<M>,
    pub offer: (TokenIdentifier<M>, BigUint<M>),
    pub bid: (TokenIdentifier<M>, BigUint<M>),
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
    #[endpoint(openOrder)]
    fn open_order(
        &self,
        token_out: TokenIdentifier,
        amount_out_min: BigUint
    ) {
        require!(token_out.is_valid_esdt_identifier(), "Invalid token out provided");
        require!(amount_out_min > 0, "Amount out min must be greater than 0");
    
        let (token_in, amount_in) = self.call_value().single_fungible_esdt();

        require!(token_in.is_valid_esdt_identifier(), "Invalid token in provided");
        require!(amount_in > 0, "Amount in must be greater than 0");

        let order = Order {
            owner: self.blockchain().get_caller(),
            offer: (token_in, amount_in),
            bid: (token_out, amount_out_min),
        };

        self.orders().push(&order);
    }

    #[endpoint(closeOrder)]
    fn close_order(
        &self,
        index: usize
    ) {
        self.orders().swap_remove(index);
    }

    #[endpoint(executeOrder)]
    fn execute_order(
        &self, 
        index: usize,
        pair_address: ManagedAddress
    ) {
        self.actual_swap_fixed_input(pair_address, index);
    }

    #[callback]
    fn my_endpoint_callback(
        &self,
        index: usize,
        owner: ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<EsdtTokenPayment>
    ) {
        match result {
            ManagedAsyncCallResult::Ok(value) => {
                let (swaped_token, _, amount_swaped) = value.into_tuple();
                self.send().direct_esdt(&owner, &swaped_token, 0, &amount_swaped);
                self.close_order(index);
            },
            ManagedAsyncCallResult::Err(_) => {
                // log the error in storage
            },
        }
    }

    #[endpoint(clearStorage)]
    fn clear_storage(&self) {
        self.orders().clear();
    }

    #[view(getOrdersCount)]
    fn get_orders_count(&self) -> usize {
        self.orders().len()
    }

    #[view(getOrderOwner)]
    fn get_order_owner(&self, index: usize) -> ManagedAddress<Self::Api> {
        self.orders().get(index).owner
    }

    #[view(getOrderOfferToken)]
    fn get_order_offer_token(&self, index: usize) -> TokenIdentifier<Self::Api> {
        self.orders().get(index).offer.0
    }

    #[view(getOrderOfferAmount)]
    fn get_order_offer_amount(&self, index: usize) -> BigUint<Self::Api> {
        self.orders().get(index).offer.1
    }

    #[view(getOrderBidToken)]
    fn get_order_bid_token(&self, index: usize) -> TokenIdentifier<Self::Api> {
        self.orders().get(index).bid.0
    }

    #[view(getOrderBidAmount)]
    fn get_order_bid_amount(&self, index: usize) -> BigUint<Self::Api> {
        self.orders().get(index).bid.1
    }

    #[view(getCurrentFunds)]
    fn get_current_funds(
        &self,
        token: &EgldOrEsdtTokenIdentifier
    ) -> BigUint {
        self.blockchain().get_sc_balance(token, 0)
    } 

    #[endpoint(claimTokens)]
    fn claim_tokens(
        &self,
        token: EgldOrEsdtTokenIdentifier,
    ) {
        let owner = self.blockchain().get_owner_address();
        let sc_balance = self.get_current_funds(&token);

        self.send().direct(&owner, &token, 0, &sc_balance);
    }

    // private

    #[proxy]
    fn pair_contract_proxy(&self, pair_address: ManagedAddress) -> swap_tokens_proxy::Proxy<Self::Api>;

    fn actual_swap_fixed_input(
        &self,
        pair_address: ManagedAddress,
        index: usize
    ) -> EsdtTokenPayment<Self::Api> {
        let order = self.orders().get(index);
        let (token_in, amount_in) = order.offer;
        let (token_out, amount_out_min) = order.bid;

        self.pair_contract_proxy(pair_address)
            .swap_tokens_fixed_input(token_out, amount_out_min)
            .with_esdt_transfer(EsdtTokenPayment::new(token_in, 0, amount_in))
            .async_call()
            .with_callback(self.callbacks().my_endpoint_callback(index, order.owner))
            .call_and_exit()
        }

    // storage

    #[view(getOrders)]
    #[storage_mapper("orders")]
    fn orders(&self) -> VecMapper<Order<Self::Api>>;
}

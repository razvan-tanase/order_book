#![no_std]

use multiversx_sc::types::BigUint;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct Order<M: ManagedTypeApi> {
    pub id: u64,
    pub owner: ManagedAddress<M>,
    pub token_in: TokenIdentifier<M>,
    pub amount_in: BigUint<M>, 
    pub token_out: TokenIdentifier<M>,
    pub limit: BigUint<M>,
}

const EXECUTION_FEE: u32 = 1000;

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
    fn init(&self) {
        self.next_id().set(0);
    }

    #[payable("*")]
    #[endpoint(openOrder)]
    fn open_order(
        &self,
        token_out: TokenIdentifier,
        limit: BigUint,
    ) {
        require!(token_out.is_valid_esdt_identifier(), "Invalid token out provided");
        require!(limit > 0, "Limit must be greater than 0");
    
        let (token_in, amount_in) = self.call_value().single_fungible_esdt();

        require!(token_in.is_valid_esdt_identifier(), "Invalid token in provided");
        require!(amount_in > 0, "Amount in must be greater than 0");

        let order = Order {
            id: self.next_id().get(),
            owner: self.blockchain().get_caller(),
            token_in,
            amount_in,
            token_out,
            limit
        };

        self.orders().push(&order);
        self.next_id().set(self.next_id().get() + 1);
    }

    #[endpoint(closeOrder)]
    fn close_order(
        &self,
        index: usize
    ) {
        let order = self.orders().get(index);

        self.send().direct_esdt(&order.owner, &order.token_in, 0, &order.amount_in);
        self.orders().swap_remove(index);
    }

    #[endpoint(executeOrder)]
    fn execute_order(
        &self, 
        index: usize,
        pair_address: ManagedAddress,
        amount_out_min: BigUint
    ) -> EsdtTokenPayment<Self::Api> {
        let order = self.orders().get(index);
        // order.executed = true;

        self.pair_contract_proxy(pair_address)
            .swap_tokens_fixed_input(order.token_out, amount_out_min)
            .with_esdt_transfer(EsdtTokenPayment::new(order.token_in, 0, order.amount_in))
            .async_call()
            .with_callback(self.callbacks().my_endpoint_callback(index, order.owner))
            .call_and_exit()
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
                
                let bot_fee = &amount_swaped / EXECUTION_FEE;
                let remaining_amount = amount_swaped - &bot_fee;
                
                self.send().direct_esdt(&self.blockchain().get_owner_address(), &swaped_token, 0, &bot_fee);
                self.send().direct_esdt(&owner, &swaped_token, 0, &remaining_amount);
                self.orders().swap_remove(index);
            },
            ManagedAsyncCallResult::Err(_) => {
                sc_panic!("Error while executing the swap");
            },
        }
    }

    // #[endpoint(changeToExecuted)]
    // fn change_to_executed(
    //     &self,
    //     index: usize
    // ) {
    //     let order = self.orders().get(index);
    //     let new_order = Order {
    //         executed: true,
    //         ..order
    //     };
    //     self.orders().set(index, &new_order);
    // }

    // #[view(isExecuted)]
    // fn is_executed(
    //     &self,
    //     index: usize
    // ) -> bool {
    //     self.orders().get(index).executed
    // }
    
    // private

    #[only_owner]
    #[endpoint(clearStorage)]
    fn clear_storage(&self) {
        for _ in 0..self.get_orders_count() {
            self.close_order(1);
        }
    }

    #[proxy]
    fn pair_contract_proxy(&self, pair_address: ManagedAddress) -> swap_tokens_proxy::Proxy<Self::Api>;

    // views

    #[view(getOrdersCount)]
    fn get_orders_count(&self) -> usize {
        self.orders().len()
    }

    #[view(getOrder)]
    fn get_order(&self, index: usize) -> Order<Self::Api> {
        self.orders().get(index)
    }

    // #[view(getCurrentFunds)]
    // fn get_current_funds(
    //     &self,
    //     token: &EgldOrEsdtTokenIdentifier
    // ) -> BigUint {
    //     self.blockchain().get_sc_balance(token, 0)
    // } 

    // #[endpoint(claimTokens)]
    // fn claim_tokens(
    //     &self,
    //     token: EgldOrEsdtTokenIdentifier,
    // ) {
    //     let owner = self.blockchain().get_owner_address();
    //     let sc_balance = self.get_current_funds(&token);

    //     self.send().direct(&owner, &token, 0, &sc_balance);
    // }

    // storage

    #[view(getNextId)]
    #[storage_mapper("nextId")]
    fn next_id(&self) -> SingleValueMapper<u64>;

    #[view(getOrders)]
    #[storage_mapper("orders")]
    fn orders(&self) -> VecMapper<Order<Self::Api>>;
}
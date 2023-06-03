#![no_std]

use multiversx_sc::types::BigUint;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct Order<M: ManagedTypeApi> {
    pub owner: ManagedAddress<M>,
    // pub offer: (TokenIdentifier<M>, BigUint<M>),
    // pub bid: (TokenIdentifier<M>, BigUint<M>),
    pub token_in: TokenIdentifier<M>,
    pub amount_in: BigUint<M>, 
    pub token_out: TokenIdentifier<M>,
    pub limit: BigUint<M>,
    pub amount_out_min: BigUint<M>,
    // pub bot_fee: BigUint<M>,
    // pub remaining_amount: BigUint<M>,
    // pub limit: BigUint<M>,
    // in bid o sa fie cantintatea, nu pretul la care sa se cumpere
}

// Valoarea lui token_out in functie de token_in este limit

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
        // ce token vreau
        token_out: TokenIdentifier,
        // la ce pret
        limit: BigUint,
        amount_out_min: BigUint
        // cat vreau sa cumper

    ) {
        require!(token_out.is_valid_esdt_identifier(), "Invalid token out provided");
        require!(limit > 0, "Limit must be greater than 0");
        require!(amount_out_min > 0, "Amount out min must be greater than 0");

        let (token_in, amount_in) = self.call_value().single_fungible_esdt();

        require!(token_in.is_valid_esdt_identifier(), "Invalid token in provided");
        require!(amount_in > 0, "Amount in must be greater than 0");

        // amount_in * limit * 0.98703 = numarul de inmultit pentru slippage + fees

        let order = Order {
            owner: self.blockchain().get_caller(),
            token_in,
            amount_in,
            token_out,
            limit,
            amount_out_min
            // bot_fee: BigUint::from(0 as u32),
            // remaining_amount: BigUint::from(0 as u32),
        };

        self.orders().push(&order);
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
        pair_address: ManagedAddress
    ) -> EsdtTokenPayment<Self::Api> {
        let order = self.orders().get(index);

        self.pair_contract_proxy(pair_address)
            .swap_tokens_fixed_input(order.token_out, order.amount_out_min)
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
            ManagedAsyncCallResult::Ok(payment) => {
                // let (swaped_token, _, amount_swaped) = payment.into_tuple();
                // let (swaped_token, amount_swaped) = self.call_value().single_fungible_esdt(); 

                // let order_fee: u32 = 10000;
                // let bot_fee = &amount_swaped / order_fee;
                // let remaining_amount = amount_swaped - &bot_fee;

                // let mut order = self.orders().get(index);
                // // order.bot_fee = bot_fee;
                // order.remaining_amount = remaining_amount;
                // self.orders().push(&mut order);

                // self.send().direct_esdt(&self.blockchain().get_owner_address(), &swaped_token, 0, &bot_fee);
                // self.send().direct_esdt(&owner, &swaped_token, 0, &amount_swaped);

                // 65 320 180
                // 65 320 18

                let (swaped_token, _, amount_swaped) = payment.into_tuple();
                self.send().direct_esdt(&owner, &swaped_token, 0, &amount_swaped);
                self.orders().swap_remove(index);

            }
            ManagedAsyncCallResult::Err(_) => {
                sc_panic!("Error while executing the swap");
            }
        }


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

    #[only_owner]
    #[endpoint(clearStorage)]
    fn clear_storage(&self) {
        // for _ in 0..self.get_orders_count() {
        //     self.close_order(1);
        // }
        self.orders().clear();
    }

    // private

    #[proxy]
    fn pair_contract_proxy(&self, pair_address: ManagedAddress) -> swap_tokens_proxy::Proxy<Self::Api>;

    // view

    #[view(getOrdersCount)]
    fn get_orders_count(&self) -> usize {
        self.orders().len()
    }

    #[view(getOrder)]
    fn get_order(&self, index: usize) -> Order<Self::Api> {
        self.orders().get(index)
    }

    #[view(getAmountMin)]
    fn get_amount_min(&self, index: usize) -> BigUint {
        self.orders().get(index).amount_out_min
    }

    // #[view(getBotFee)]
    // fn get_bot_fee(&self, index: usize) -> BigUint {
    //     self.orders().get(index).bot_fee
    // }

    // #[view(getRemainingAmount)]
    // fn get_remaining_amount(&self, index: usize) -> BigUint {
    //     self.orders().get(index).remaining_amount
    // }

    // storage 

    #[view(getOrders)]
    #[storage_mapper("orders")]
    fn orders(&self) -> VecMapper<Order<Self::Api>>;
}

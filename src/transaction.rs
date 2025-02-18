use crate::config::{ADMIN_PUBKEY, CONFIG};
use crate::player::{ExchangePlayer};
use crate::settlement::{SettlementInfo};
use crate::state::STATE;
use crate::StorageData;
use std::slice::IterMut;
use std::u64;
use serde::Serialize;
use zkwasm_rest_abi::{WithdrawInfo, MERKLE_MAP};
use zkwasm_rust_sdk::require;
use crate::event::{EVENT_MARKET, EVENT_ORDER, EVENT_TOKEN, EVENT_TRADE, finalize_events, get_event, init_events};

pub struct Transaction {
    pub nonce: u64,
    pub data: Data,
}

const ADD_TOKEN: u64 = 1;
const ADD_MARKET: u64 = 2;
const DEPOSIT_TOKEN: u64 = 3;
const REGISTER_PLAYER: u64 = 4;
const ADD_LIMIT_ORDER: u64 = 5;

const ADD_MARKET_ORDER: u64 = 6;
const CANCEL_ORDER: u64 = 7;
const CLOSE_MARKET: u64 = 8;
const TRANSFER: u64 = 9;
const WITHDRAW: u64 = 10;
const ADD_TRADE: u64 = 11;

const UPDATE_TOKEN: u64 = 12;

// TODO fix error number
const ERROR_PLAYER_ALREADY_EXIST: u32 = 1;
const ERROR_TOKEN_ALREADY_EXIST: u32 = 2;
const ERROR_TOKEN_NOT_EXIST: u32 = 3;
const ERROR_PLAYER_NOT_EXIST: u32 = 4;
const ERROR_MARKET_NOT_EXIST: u32 = 5;

const ERROR_BALANCE_NOT_ENOUGH: u32 = 6;

const ERROR_PLAYER_ORDER_NOT_MATCH: u32 = 7;

const ERROR_ORDER_NOT_LIVE: u32 = 8;

const ERROR_ORDER_NOT_EXIT: u32 = 9;
const ERROR_TRADE_A_BUY_B_SELL: u32 = 10;
const ERROR_LIMIT_ORDER_PRICE_NOT_MATCH: u32 = 11;
const ERROR_SAME_PID: u32 = 12;
const ERROR_A_AND_B_ORDER_BOTH_MARKET: u32 = 13;
const ERROR_ORDER_MARKET_NOT_MATCH: u32 = 14;
const ERROR_BALANCE_NOT_MATCH: u32 = 15;
const ERROR_OVERFLOW: u32 = 16;
const ERROR_AMOUNT : u32 = 17;

const ERROR_PLAYER_IS_NOT_ADMIN: u32 = 0xffffffff;

const COMMON_PREFIX: u64 = 0xffffffffffffffff;
const ORDER_PREFIX: u64 = 0xfffffffffffffffd;
const TRADE_PREFIX: u64 = 0xfffffffffffffffc;
const TOKEN_STORE_PREFIX: u64 = 1;
const MARKET_STORE_PREFIX: u64 = 1;

pub fn u64_array_to_address(arr: &[u64; 3]) -> [u8; 20] {
    let mut address = ((arr[0] >> 32) as u32).to_le_bytes().to_vec();
    address.extend_from_slice(&arr[1].to_le_bytes());
    address.extend_from_slice(&arr[2].to_le_bytes());
    address.as_slice().try_into().unwrap()
}

pub fn safe_mul(a: u64, b: u64) -> Option<u64> {
    let a = a as u128;
    let b = b as u128;
    let precision = CONFIG.precision as u128;
    let result = a.checked_mul(b).unwrap();
    let result = result/precision;
    if result > u64::MAX as u128 {
        None
    } else {
        Some(result as u64)
    }
}

impl Transaction {
    pub fn decode_error(e: u32) -> &'static str {
        match e {
            ERROR_PLAYER_ALREADY_EXIST => "PlayerAlreadyExist",
            ERROR_TOKEN_ALREADY_EXIST => "TokenAlreadyExist",
            ERROR_TOKEN_NOT_EXIST => "TokenNotExist",
            ERROR_PLAYER_NOT_EXIST => "PlayerNotExist",
            ERROR_MARKET_NOT_EXIST => "MarketNotExist",
            ERROR_BALANCE_NOT_ENOUGH => "BalanceNotEnough",
            ERROR_PLAYER_ORDER_NOT_MATCH => "PlayerOrderNotMatch",
            ERROR_ORDER_NOT_LIVE => "OrderNotLive",
            ERROR_ORDER_NOT_EXIT => "OrderNotExist",
            ERROR_TRADE_A_BUY_B_SELL => "TradeABuyBSell",
            ERROR_LIMIT_ORDER_PRICE_NOT_MATCH => "LimitOrderPriceNotMatch",
            ERROR_SAME_PID => "SamePid",
            ERROR_A_AND_B_ORDER_BOTH_MARKET => "AAndBOrderBothMarket",
            ERROR_ORDER_MARKET_NOT_MATCH => "OrderMarketNotMatch",
            ERROR_BALANCE_NOT_MATCH => "BalanceNotMatch",
            ERROR_OVERFLOW => "Overflow",
            ERROR_AMOUNT => "Amount",
            // todo add other error
            _ => "Unknown",
        }
    }
    pub fn decode(params: &[u64]) -> Self {
        zkwasm_rust_sdk::dbg!("decode \n");
        let str = format!("{:?}", params);
        zkwasm_rust_sdk::dbg!("{}\n", str);
        zkwasm_rust_sdk::dbg!("\n");
        let command = params[0] & 0xff;
        let nonce = params[0] >> 16;
        let data = match command {
            ADD_TOKEN => {
                unsafe {
                    require(params.len() == 5);
                };
                let token_idx = params[1] as u32;
                let address = u64_array_to_address(&[params[2], params[3], params[4]]);
                Data::AddToken(AddTokenParams { token_idx, address })
            }
            UPDATE_TOKEN => {
                unsafe {
                    require(params.len() == 5);
                };
                let token_idx = params[1] as u32;
                let address = u64_array_to_address(&[params[2], params[3], params[4]]);
                Data::UpdateToken(UpdateTokenParams { token_idx, address })
            }
            ADD_MARKET => {
                unsafe {
                    require(params.len() == 4);
                };
                let token_a = params[1] as u32;
                let token_b = params[2] as u32;
                let last_deal_price = params[3];
                Data::AddMarket(AddMarketParams { token_a, token_b, last_deal_price })
            }

            DEPOSIT_TOKEN => {
                unsafe {
                    require(params.len() == 5);
                };
                zkwasm_rust_sdk::dbg!("deposit token\n");
                Data::Deposit(DepositParams {
                    pid_1: params[1],
                    pid_2: params[2],
                    token_idx: params[3] as u32,
                    amount: params[4],
                })
            }

            REGISTER_PLAYER =>{
                unsafe {
                    require(params.len() == 1);
                };
                Data::RegisterPlayer
            },

            ADD_LIMIT_ORDER => {
                unsafe {
                    require(params.len() == 5);
                };
                Data::AddLimitOrder(AddLimitOrderParams {
                market_id: params[1],
                flag: params[2],
                limit_price: params[3],
                amount: params[4],
            })
            },
            ADD_MARKET_ORDER => {
                unsafe {
                    require(params.len() == 5);
                };
                Data::AddMarketOrder(AddMarketOrderParams {
                    market_id: params[1],
                    flag: params[2],
                    b_token_amount: params[3],
                    a_token_amount: params[4],
                })
            },
            CANCEL_ORDER => {
                unsafe {
                    require(params.len() == 2);
                };
                Data::CancelOrder(CancelOrderParams {
                    order_id: params[1],
                })
            },
            CLOSE_MARKET => {
                unsafe {
                    require(params.len() == 2);
                };
                Data::CloseMarket(CloseMarketParams {
                market_id: params[1],
            })
            },
            TRANSFER => {
                unsafe {
                    require(params.len() == 5);
                };
                Data::Transfer(TransferParams {
                    pid_1: params[1],
                    pid_2: params[2],
                    token_idx: params[3] as u32,
                    amount: params[4],
                })
            },
            WITHDRAW => {
                unsafe {
                    require(params.len() == 6);
                };
                Data::Withdraw(WithdrawParams {
                    token_idx: params[1] as u32,
                    to_address: u64_array_to_address(&[params[2], params[3], params[4]]),
                    amount: params[5],
                })
            },
            ADD_TRADE => {
                unsafe {
                    require(params.len() == 5);
                };
                Data::AddTrade(AddTradeParams {
                    a_order_id: params[1],
                    b_order_id: params[2],
                    a_actual_amount: params[3],
                    b_actual_amount: params[4],
                })
            },
            _ => {
                zkwasm_rust_sdk::dbg!("unknown command\n");
                Data::RegisterPlayer
            }
        };
        Transaction { nonce, data }
    }
    pub fn install_player(&self, pkey: &[u64; 4]) -> u32 {
        zkwasm_rust_sdk::dbg!("install \n");
        let pid = ExchangePlayer::pkey_to_pid(pkey);
        let player = ExchangePlayer::get_from_pid(&pid);
        match player {
            Some(_) => ERROR_PLAYER_ALREADY_EXIST,
            None => {
                let mut player = ExchangePlayer::new_from_pid(pid);
                player.check_and_inc_nonce(self.nonce);
                player.store();
                unsafe {STATE.tick()};
                0
            }
        }
    }

    pub fn is_admin(pkey: &[u64; 4]) -> bool {
        *pkey == *ADMIN_PUBKEY
    }

    pub fn process(&self, pkey: &[u64; 4], rand: &[u64; 4]) -> Vec<u64> {
        unsafe {
          init_events();
        };
        let ret = self.process_inner(pkey, rand);
        unsafe {
            finalize_events(ret);
        };
        unsafe {return get_event().clone();}
    }

    pub fn process_inner(&self, pkey: &[u64; 4], _rand: &[u64; 4]) -> u32 {
        let pid_1 = pkey[1];
        let pid_2 = pkey[2];
        let nonce = self.nonce;
        zkwasm_rust_sdk::dbg!("process pkey: {} {}, nonce: {}\n", pid_1, pid_2, nonce);
        match self.data {
            Data::RegisterPlayer => {
                self.install_player(pkey)
            },
            Data::AddToken(ref params) => {
                if params.token_idx > 255 {
                    zkwasm_rust_sdk::dbg!("token idx overflow\n");
                    return ERROR_OVERFLOW;
                }

                if !Self::is_admin(pkey) {
                    zkwasm_rust_sdk::dbg!("you are not admin\n");
                    return ERROR_PLAYER_IS_NOT_ADMIN;
                }
                let player = ExchangePlayer::get_and_check_nonce(&[pid_1, pid_2], self.nonce);

                zkwasm_rust_sdk::dbg!("add token\n");
                if Token::load(params.token_idx).is_some() {
                    return ERROR_TOKEN_ALREADY_EXIST;
                }
                let token = Token::new(params.token_idx, params.address);
                token.store();
                token.add_event();
                player.store();
                unsafe { STATE.tick() };
                // TODO emit event
                0
            }
            Data::UpdateToken(ref params) => {
                if params.token_idx > 255 {
                    zkwasm_rust_sdk::dbg!("token idx overflow\n");
                    return ERROR_OVERFLOW;
                }

                if !Self::is_admin(pkey) {
                    zkwasm_rust_sdk::dbg!("you are not admin\n");
                    return ERROR_PLAYER_IS_NOT_ADMIN;
                }
                let player = ExchangePlayer::get_and_check_nonce(&[pid_1, pid_2], self.nonce);

                zkwasm_rust_sdk::dbg!("update token\n");
                let token = Token::load(params.token_idx);
                if token.is_none() {
                    return ERROR_TOKEN_NOT_EXIST;
                }
                let mut token = token.unwrap();
                token.address = params.address;
                token.store();
                token.add_event();
                player.store();
                unsafe { STATE.tick() };
                0
            }
            Data::AddMarket(ref params) => {
                zkwasm_rust_sdk::dbg!("add market\n");
                if !Self::is_admin(pkey) {
                    zkwasm_rust_sdk::dbg!("you are not admin\n");
                    return ERROR_PLAYER_IS_NOT_ADMIN;
                }
                let a_exist = Token::load(params.token_a).is_some();
                let b_exist = Token::load(params.token_b).is_some();
                zkwasm_rust_sdk::dbg!("token_a {} \n", a_exist);
                zkwasm_rust_sdk::dbg!("token_b {} \n", b_exist);
                unsafe {
                    require(a_exist);
                    require(b_exist);
                }
                let player = ExchangePlayer::get_and_check_nonce(&[pid_1, pid_2], self.nonce);
                let new_market_id = unsafe { STATE.get_new_market_id() };
                let market = Market::new(new_market_id, params.token_a, params.token_b, params.last_deal_price);
                market.store();
                market.add_event();
                player.store();
                // TODO emit event
                unsafe { STATE.tick() };
                0
            }
            Data::Deposit(ref params) => {
                zkwasm_rust_sdk::dbg!("deposit\n");
                if !Self::is_admin(pkey) {
                    zkwasm_rust_sdk::dbg!("you are not admin\n");
                    return ERROR_PLAYER_IS_NOT_ADMIN;
                }
                let admin = ExchangePlayer::get_and_check_nonce(&ExchangePlayer::pkey_to_pid(pkey), self.nonce);
                let pid = [params.pid_1, params.pid_2];
                zkwasm_rust_sdk::dbg!("Deposit: {} {}\n", pid_1, pid_2);
                let mut player = ExchangePlayer::get_from_pid(&pid).unwrap_or_else(
                    || {
                        let player = ExchangePlayer::new_from_pid(pid);
                        player
                    },

                );

                let token = Token::load(params.token_idx);
                if token.is_none() {
                    zkwasm_rust_sdk::dbg!("deposit token not exist\n");
                    return ERROR_TOKEN_NOT_EXIST;
                }

                let position = player.data.load_position(params.token_idx, &pid);
                //todo checkout overflow?
                if !position.inc_balance(params.amount) {
                    zkwasm_rust_sdk::dbg!("deposit overflow\n");
                    return ERROR_OVERFLOW;
                }
                player.data.store_positions(&player.player_id);
                player.store();
                admin.store();
                unsafe { STATE.tick() };
                0
            }
            Data::AddLimitOrder(ref params) => {
                zkwasm_rust_sdk::dbg!("add limit order\n");

                match self.add_limit_order(&[pid_1, pid_2], params) {
                    Ok(value) => {
                        unsafe { STATE.tick() };
                        value
                    },
                    Err(value) => return value,
                }
            }

            Data::AddMarketOrder(ref params) => {
                zkwasm_rust_sdk::dbg!("add market order\n");
                match self.add_market_order(&[pid_1, pid_2], params) {
                    Ok(value) => {
                        unsafe { STATE.tick() };
                        value
                    },

                    Err(value) => return value,
                }
            }
            Data::CancelOrder(ref params) => {
                zkwasm_rust_sdk::dbg!("cancel order\n");
                match self.cancel_order(&[pid_1, pid_2], params) {
                    Ok(value) => {
                        unsafe { STATE.tick() };
                        value
                    },

                    Err(value) => return value,
                }
            }
            Data::CloseMarket(ref params) => {
                zkwasm_rust_sdk::dbg!("close market\n");
                if !Self::is_admin(pkey) {
                    zkwasm_rust_sdk::dbg!("you are not admin\n");
                    return ERROR_PLAYER_IS_NOT_ADMIN;
                }
                let player = ExchangePlayer::get_and_check_nonce(&[pid_1, pid_2], self.nonce);
                let market = Market::load(params.market_id);
                if market.is_none() {
                    zkwasm_rust_sdk::dbg!("market not exist\n");
                    return ERROR_MARKET_NOT_EXIST;
                }
                let mut market = market.unwrap();
                market.status = MARKET_STATUS_CLOSE;
                market.store();
                market.add_event();
                player.store();
                unsafe { STATE.tick() };
                0
            }

            Data::Transfer(ref params) => {
                zkwasm_rust_sdk::dbg!("transfer\n");
                if !Self::is_admin(pkey) {
                    zkwasm_rust_sdk::dbg!("you are not admin\n");
                    return ERROR_PLAYER_IS_NOT_ADMIN;
                }
                let mut player_1 = ExchangePlayer::get_and_check_nonce(&[pid_1, pid_2], self.nonce);
                let player_2_opt = ExchangePlayer::get_from_pid(&[params.pid_1, params.pid_2]);
                let mut player_2 = player_2_opt.unwrap_or_else(|| {
                    let player = ExchangePlayer::new_from_pid([params.pid_1, params.pid_2]);
                    player
                });

                if player_1.player_id == player_2.player_id {
                    zkwasm_rust_sdk::dbg!("transfer, player a and player b is same\n");
                    return ERROR_SAME_PID;
                }

                let token = Token::load(params.token_idx);
                if token.is_none() {
                    zkwasm_rust_sdk::dbg!("transfer, token not exist\n");
                    return ERROR_TOKEN_NOT_EXIST;
                }
                let position_1 = player_1
                    .data
                    .load_position(params.token_idx, &[pid_1, pid_2]);
                let position_2 = player_2
                    .data
                    .load_position(params.token_idx, &[params.pid_1, params.pid_2]);

                if !position_1.dec_balance(params.amount) {
                    zkwasm_rust_sdk::dbg!("transfer, balance not enough\n");
                    return ERROR_BALANCE_NOT_ENOUGH;

                };
                if !position_2.inc_balance(params.amount) {
                    zkwasm_rust_sdk::dbg!("transfer, balance overflow\n");
                    return ERROR_OVERFLOW;
                };

                player_1.data.store_positions(&player_1.player_id);
                player_1.store();

                player_2.data.store_positions(&player_2.player_id);
                player_2.store();

                unsafe { STATE.tick() };
                0
            }
            Data::Withdraw(ref params) => {
                zkwasm_rust_sdk::dbg!("withdraw\n");
                let mut player = ExchangePlayer::get_and_check_nonce(&[pid_1, pid_2], self.nonce);

                let token = Token::load(params.token_idx);
                if token.is_none() {
                    zkwasm_rust_sdk::dbg!("withdraw, token not exist\n");
                    return ERROR_TOKEN_NOT_EXIST;
                }
                let position = player
                    .data
                    .load_position(params.token_idx, &[pid_1, pid_2]);
                if position.balance < params.amount {
                    zkwasm_rust_sdk::dbg!("withdraw, balance not enough\n");
                    return ERROR_BALANCE_NOT_ENOUGH;
                }
                let withdraw_info = WithdrawInfo {
                    feature: params.token_idx<<8,
                    address: params.to_address,
                    amount: params.amount/CONFIG.precision,
                };
                SettlementInfo::append_settlement(withdraw_info);
                position.dec_balance(params.amount);
                position.store(params.token_idx, &[pid_1, pid_2]);
                player.store();
                unsafe { STATE.tick() };
                0
            }
            Data::AddTrade(ref params) => match self.add_trade(pkey, params) {
                Ok(value) => {
                    unsafe { STATE.tick() };
                    value
                },
                Err(value) => return value,
            }
        }
    }

    pub fn add_trade(&self, pkey: &[u64; 4], params: &AddTradeParams) -> Result<u32, u32> {
        zkwasm_rust_sdk::dbg!("add trade\n");

        if !Self::is_admin(pkey) {
            zkwasm_rust_sdk::dbg!("you are not admin\n");
            return Err(ERROR_PLAYER_IS_NOT_ADMIN);
        }

        let player = ExchangePlayer::get_and_check_nonce(&ExchangePlayer::pkey_to_pid(pkey), self.nonce);

        let a_order = Order::load(params.a_order_id);
        let b_order = Order::load(params.b_order_id);
        if a_order.is_none() || b_order.is_none() {
            zkwasm_rust_sdk::dbg!("add trade, a order or b order is none\n");
            return Err(ERROR_ORDER_NOT_EXIT);
        }
        let mut a_order = a_order.unwrap();
        let mut b_order = b_order.unwrap();
        if !(a_order.is_live() && b_order.is_live()) {
            zkwasm_rust_sdk::dbg!("add trade, a order or b order is not live\n");
            return Err(ERROR_ORDER_NOT_LIVE);
        }
        let market = Market::load(a_order.market_id);
        if market.is_none() {
            zkwasm_rust_sdk::dbg!("add trade, a order or b order is not exist\n");
            return Err(ERROR_MARKET_NOT_EXIST);
        }
        let mut market = market.unwrap();
        if market.is_close() {
            zkwasm_rust_sdk::dbg!("add trade, a order or b order is closed\n");
            return Err(ERROR_MARKET_NOT_EXIST);
        }

        if a_order.market_id != b_order.market_id {
            zkwasm_rust_sdk::dbg!("add trade, a order and b order is not in same market\n");
            return Err(ERROR_ORDER_MARKET_NOT_MATCH);
        }

        if !(a_order.is_buy() && b_order.is_sell()) {
            zkwasm_rust_sdk::dbg!("add trade, a order is not buy or b order is not sell\n");
            return Err(ERROR_TRADE_A_BUY_B_SELL);
        }
        if a_order.pid == b_order.pid {
            zkwasm_rust_sdk::dbg!("add trade, a order and b order is from same user\n");
            return Err(ERROR_SAME_PID);
        }
        let player_a = ExchangePlayer::get_from_pid(&a_order.pid);
        let player_b = ExchangePlayer::get_from_pid(&b_order.pid);
        if player_a.is_none() || player_b.is_none() {
            zkwasm_rust_sdk::dbg!("add trade, player a or player b is none\n");
            return Err(ERROR_PLAYER_NOT_EXIST);
        }
        let mut player_a = player_a.unwrap();
        let mut player_b = player_b.unwrap();

        if a_order.is_market_order() && b_order.is_market_order() {
            // 第四种情况 a order, b order 都是 market order
            zkwasm_rust_sdk::dbg!("balance a order and b order is market order\n");
            return Err(ERROR_A_AND_B_ORDER_BOTH_MARKET);
        }

        // 第一种情况 a order, b order 都是 limit order
        if a_order.is_limit_order() && b_order.is_limit_order() {
            if a_order.price <= b_order.price {
                zkwasm_rust_sdk::dbg!("both a and b is limit order but not buy price >= sell price \n");
                return Err(ERROR_LIMIT_ORDER_PRICE_NOT_MATCH);
            }
        }
        let price = if a_order.is_limit_order() {
            if b_order.is_limit_order() {
                b_order.price
            } else {
                a_order.price
            }
        } else {
            b_order.price
        };

        let a_amount = match safe_mul(price, params.b_actual_amount) {
            Some(v) => v,
            None => {
                zkwasm_rust_sdk::dbg!("price * b_actual_amount overflow\n");
                return Err(ERROR_OVERFLOW);
            }
        };

        if a_amount != params.a_actual_amount {
            zkwasm_rust_sdk::dbg!(
                "price*b_actual_amount not equal a_actual_amount\n"
            );
            return Err(ERROR_LIMIT_ORDER_PRICE_NOT_MATCH);
        }


        let a_cost = params.a_actual_amount;
        let b_cost = params.b_actual_amount;
        if !(a_order.lock_balance >= params.a_actual_amount && b_order.lock_balance >= params.b_actual_amount) {
            zkwasm_rust_sdk::dbg!("balance not match\n");
            return Err(ERROR_BALANCE_NOT_MATCH);
        }
        if a_order.is_limit_order() && b_order.is_limit_order() {
            a_order.lock_balance -= params.a_actual_amount;
            b_order.lock_balance -= params.b_actual_amount;
            a_order.already_deal_amount += params.b_actual_amount;
            b_order.already_deal_amount += params.b_actual_amount;
        } else if a_order.is_limit_order() && b_order.is_market_order() {
            // 第二种情况 a order 是 limit order, b order 是 market order
            // todo check price
            a_order.lock_balance -= params.a_actual_amount;
            b_order.lock_balance -= params.b_actual_amount;
            a_order.already_deal_amount += params.b_actual_amount;
            if b_order.a_token_amount != 0 {
                b_order.already_deal_amount += params.a_actual_amount;
            } else {
                b_order.already_deal_amount += params.b_actual_amount;
            }
        } else if a_order.is_market_order() && b_order.is_limit_order() {
            // 第三种情况 a order 是 market order, b order 是 limit order
            // todo check price
            a_order.lock_balance -= params.a_actual_amount;
            b_order.lock_balance -= params.b_actual_amount;
            if a_order.a_token_amount != 0 {
                a_order.already_deal_amount += params.a_actual_amount;
            } else {
                a_order.already_deal_amount += params.b_actual_amount;
            }
            b_order.already_deal_amount += params.b_actual_amount;
        }

        {
            let player_a_token_a_position =
                player_a.data.load_position(market.token_a, &a_order.pid);
            player_a_token_a_position.dec_lock_balance(a_cost);
        }
        let player_a_token_b_position =
            player_a.data.load_position(market.token_b, &a_order.pid);
        {
            let player_b_token_a_position =
                player_b.data.load_position(market.token_a, &b_order.pid);
            if !player_b_token_a_position.inc_balance(a_cost) {
                zkwasm_rust_sdk::dbg!("balance overflow\n");
                return Err(ERROR_OVERFLOW);
            };
        }
        let player_b_token_b_position =
            player_b.data.load_position(market.token_b, &b_order.pid);
        if !player_a_token_b_position.inc_balance(b_cost) {
            zkwasm_rust_sdk::dbg!("balance overflow\n");
            return Err(ERROR_OVERFLOW);
        };
        player_b_token_b_position.dec_lock_balance(b_cost);

        //process fee
        {
            let fee_token_idx = CONFIG.fee_token_idx;
            if a_order.lock_fee > 0 {
                unsafe {
                    STATE.total_fee += a_order.lock_fee;
                }
                let player_a_fee_position =
                    player_a.data.load_position(fee_token_idx, &a_order.pid);
                player_a_fee_position.dec_lock_balance(a_order.lock_fee);
                a_order.lock_fee = 0;
            }

            if b_order.lock_fee > 0 {
                unsafe {
                    STATE.total_fee += b_order.lock_fee;
                }
                let player_b_fee_position =
                    player_b.data.load_position(fee_token_idx, &b_order.pid);
                player_b_fee_position.dec_lock_balance(b_order.lock_fee);
                b_order.lock_fee = 0;
            }
        }

        a_order.update_status();
        b_order.update_status();

        if a_order.status == Order::STATUS_MATCH {
            // check profit
            if a_order.lock_balance > 0 {
                let player_a_token_a_position =
                    player_a.data.load_position(market.token_a, &a_order.pid);
                player_a_token_a_position.dec_lock_balance(a_order.lock_balance);
                player_a_token_a_position.inc_balance(a_order.lock_balance);
                a_order.lock_balance = 0;
            }
        }
        player_a.data.store_positions(&player_a.player_id);
        player_a.data.add_positions_event(&player_a.player_id);
        player_b.data.store_positions(&player_b.player_id);
        player_b.data.add_positions_event(&player_b.player_id);
        let trace_id = unsafe { STATE.get_new_trade_id() };
        let trade = Trade::new(
            trace_id,
            a_order.id,
            b_order.id,
            params.a_actual_amount,
            params.b_actual_amount,
        );
        market.last_deal_price = price;
        market.add_event();
        market.store();
        trade.store();
        trade.add_event(a_order.market_id);
        a_order.store();
        a_order.add_event();
        b_order.store();
        b_order.add_event();
        player.store();
        Ok(0)
    }

    pub fn add_limit_order(&self, pid: &[u64; 2], params: &AddLimitOrderParams) -> Result<u32, u32> {
        let mut player = ExchangePlayer::get_and_check_nonce(pid, self.nonce);

        let market = Market::load(params.market_id);
        if market.is_none() {
            zkwasm_rust_sdk::dbg!("add limit order, market is none\n");
            return Err(ERROR_MARKET_NOT_EXIST);
        }

        if market.clone().unwrap().is_close() {
            zkwasm_rust_sdk::dbg!("add limit order, market is closed\n");
            return Err(ERROR_MARKET_NOT_EXIST);
        }

        let token_idx: u32;
        let cost: u64;
        if params.flag == Order::FLAG_BUY as u64 {
            token_idx = market.unwrap().token_a;
            cost = match safe_mul(params.amount, params.limit_price) {
                Some(v) => v,
                None => {
                    zkwasm_rust_sdk::dbg!("add limit order, amount * limit_price overflow\n");
                    return Err(ERROR_OVERFLOW);
                }
            };
        } else {
            token_idx = market.unwrap().token_b;
            cost = params.amount;
        };

        {
            let position = player.data.load_position(token_idx, pid);
            if position.balance < cost {
                zkwasm_rust_sdk::dbg!("add limit order, balance is not enough\n");
                return Err(ERROR_BALANCE_NOT_ENOUGH);
            }
            if !position.dec_balance(cost) {
                zkwasm_rust_sdk::dbg!("add limit order, balance is not enough\n");
                return Err(ERROR_BALANCE_NOT_ENOUGH);

            };
            if !position.inc_lock_balance(cost) {
                zkwasm_rust_sdk::dbg!("add limit order, lock balance is overflow\n");
                return Err(ERROR_BALANCE_NOT_ENOUGH)
            };
        }


        let fee_token_idx = CONFIG.fee_token_idx;
        let fee = CONFIG.fee;
        let fee_cost: u64 = fee;
        {
            let fee_position = player.data.load_position(fee_token_idx, pid);
            if !fee_position.dec_balance(fee_cost) {
                zkwasm_rust_sdk::dbg!("add limit order, fee balance is not enough\n");
                return Err(ERROR_BALANCE_NOT_ENOUGH);

            };
            if !fee_position.inc_lock_balance(fee_cost) {
                zkwasm_rust_sdk::dbg!("add limit order, fee lock balance is overflow\n");
                return Err(ERROR_BALANCE_NOT_ENOUGH)
            };
        }

        let order_id = unsafe { STATE.get_new_order_id() };
        let order = Order::new(
            order_id,
            Order::TYPE_LIMIT,
            Order::STATUS_LIVE,
            *pid,
            params.market_id,
            params.flag as u8,
            cost,
            fee_cost,
            params.limit_price,
            params.amount,
            0,
            0,
        );
        player.data.store_positions(pid);
        player.data.add_positions_event(pid);
        order.store();
        order.add_event();
        player.store();
        Ok(0)
    }

    pub fn add_market_order(&self, pid: &[u64; 2], params: &AddMarketOrderParams) -> Result<u32, u32> {
        let mut player = ExchangePlayer::get_and_check_nonce(pid, self.nonce);
        let market = Market::load(params.market_id);
        if market.is_none() {
            zkwasm_rust_sdk::dbg!("add market order, market is none\n");
            return Err(ERROR_MARKET_NOT_EXIST);
        }

        if market.clone().unwrap().is_close() {
            zkwasm_rust_sdk::dbg!("add market order, market is closed\n");
            return Err(ERROR_MARKET_NOT_EXIST);
        }
        let market = market.unwrap();

        if params.b_token_amount != 0 && params.a_token_amount != 0 {
            zkwasm_rust_sdk::dbg!("add market order, both a and b token amount is not 0\n");
            return Err(ERROR_AMOUNT);
        }

        let mut token_idx: u32 = u32::MAX;
        let mut cost: u64 = 0;
        if params.flag == Order::FLAG_BUY as u64 {
            if params.a_token_amount != 0 {
                token_idx = market.token_a;
                cost = params.a_token_amount;
            } else {
                token_idx = market.token_a;
                let r = safe_mul(params.b_token_amount, market.last_deal_price);
                if r.is_none() {
                    return Err(ERROR_OVERFLOW);
                }
                cost = r.unwrap() * 2;
            }
        } else if params.flag == Order::FLAG_SELL as u64 {
            if params.b_token_amount != 0 {
                token_idx = market.token_b;
                cost = params.b_token_amount;
            } else {
                token_idx = market.token_b;
                cost = (params.a_token_amount*2* CONFIG.precision) / market.last_deal_price;
            }
        };

        let position = player.data.load_position(token_idx, pid);
            let balance = position.balance;
            zkwasm_rust_sdk::dbg!("add market order, token_idx {} balance {} is not enough, cost {}\n", token_idx, balance, cost);
        // todo check overflow
        if position.balance < cost {

            return Err(ERROR_BALANCE_NOT_ENOUGH);
        }

        if !position.dec_balance(cost) {
            zkwasm_rust_sdk::dbg!("add market order, balance is not enough\n");
            return Err(ERROR_BALANCE_NOT_ENOUGH);
        };
        if !position.inc_lock_balance(cost) {
            zkwasm_rust_sdk::dbg!("add market order, lock balance is overflow\n");
            return Err(ERROR_BALANCE_NOT_ENOUGH)
        };

        let fee_token_idx = CONFIG.fee_token_idx;
        let fee = CONFIG.fee;
        let fee_cost: u64 = fee;
        {
            let fee_position = player.data.load_position(fee_token_idx, pid);
            if !fee_position.dec_balance(fee_cost) {
                zkwasm_rust_sdk::dbg!("add market order, fee balance is not enough\n");
                return Err(ERROR_BALANCE_NOT_ENOUGH);
            };
            if !fee_position.inc_lock_balance(fee_cost) {
                zkwasm_rust_sdk::dbg!("add market order, fee lock balance is overflow\n");
                return Err(ERROR_BALANCE_NOT_ENOUGH)
            };
        }

        let order_id = unsafe { STATE.get_new_order_id() };
        let order = Order::new(
            order_id,
            Order::TYPE_MARKET,
            Order::STATUS_LIVE,
            *pid,
            params.market_id,
            params.flag as u8,
            cost,
            fee_cost,
            0,
            params.b_token_amount,
            params.a_token_amount,
            0,
        );
        player.data.store_positions(pid);
        player.data.add_positions_event(pid);
        player.store();
        order.store();
        order.add_event();
        Ok(0)
    }

    pub fn cancel_order(&self, pid: &[u64; 2], params: &CancelOrderParams) -> Result<u32, u32> {
        let order = Order::load(params.order_id);
        if order.is_none() {
            zkwasm_rust_sdk::dbg!("cancel order, order is none\n");
            return Err(ERROR_PLAYER_NOT_EXIST);
        }
        let order = order.unwrap();
        if order.pid != *pid {
            zkwasm_rust_sdk::dbg!("cancel order, order is not owned by this player\n");
            return Err(ERROR_PLAYER_ORDER_NOT_MATCH);
        }
        if !order.is_live() {
            zkwasm_rust_sdk::dbg!("cancel order, order is not live\n");
            return Err(ERROR_ORDER_NOT_LIVE);
        }
        let market = Market::load(order.market_id);
        if market.is_none() {
            zkwasm_rust_sdk::dbg!("cancel order, market is none\n");
            return Err(ERROR_MARKET_NOT_EXIST);
        }
        let token_idx: u32;
        let lock_balance = order.lock_balance;
        if order.flag == Order::FLAG_BUY as u8 {
            token_idx = market.unwrap().token_a;
        } else {
            token_idx = market.unwrap().token_b;
        };

        let mut player = ExchangePlayer::get_and_check_nonce(pid, self.nonce);
        {
            let position = player
                .data
                .load_position(token_idx, pid);
            if !position.inc_balance(lock_balance) {
                zkwasm_rust_sdk::dbg!("cancel order, balance is overflow\n");
                return Err(ERROR_OVERFLOW);
            };
            if !position.dec_lock_balance(lock_balance) {
                zkwasm_rust_sdk::dbg!("cancel order, lock balance is overflow\n");
                return Err(ERROR_OVERFLOW);
            };
        }

        if order.lock_fee > 0 {
            let fee_token_idx = CONFIG.fee_token_idx;
            let fee = order.lock_fee;
            let fee_position = player.data.load_position(fee_token_idx, pid);
            if !fee_position.inc_balance(fee) {
                zkwasm_rust_sdk::dbg!("cancel order, fee balance is overflow\n");
                return Err(ERROR_OVERFLOW);
            };
            if !fee_position.dec_lock_balance(fee) {
                zkwasm_rust_sdk::dbg!("cancel order, fee lock balance is overflow\n");
                return Err(ERROR_OVERFLOW);
            };
        }

        let mut order = order;
        order.status = Order::STATUS_CANCEL;
        order.lock_balance = 0;
        order.store();
        order.add_event();
        player.data.store_positions(&player.player_id);
        player.data.add_positions_event(&player.player_id);
        player.store();
        Ok(0)
    }
}

#[derive(Debug, Clone)]
pub struct AddMarketParams {
    pub token_a: u32,
    pub token_b: u32,
    pub last_deal_price: u64,
}

#[derive(Debug, Clone)]
pub struct CloseMarketParams {
    pub market_id: u64,
}

#[derive(Debug, Clone)]
pub struct CancelOrderParams {
    pub order_id: u64,
}

#[derive(Debug, Clone)]
pub struct WithdrawParams {
    pub token_idx: u32,
    pub to_address: [u8; 20],
    pub amount: u64,
}

#[derive(Debug, Clone)]
pub struct AddLimitOrderParams {
    pub market_id: u64,
    pub flag: u64,
    pub limit_price: u64,
    pub amount: u64,
}

#[derive(Debug, Clone)]
pub struct AddMarketOrderParams {
    pub market_id: u64,
    pub flag: u64,
    pub b_token_amount: u64,
    pub a_token_amount: u64,
}

#[derive(Debug, Clone)]
pub struct AddTradeParams {
    pub a_order_id: u64,
    pub b_order_id: u64,
    pub a_actual_amount: u64,
    pub b_actual_amount: u64,
}

#[derive(Debug, Clone)]
pub struct TransferParams {
    pub pid_1: u64,
    pub pid_2: u64,
    pub token_idx: u32,
    pub amount: u64,
}

type DepositParams = TransferParams;

#[derive(Debug, Clone)]
pub struct AddTokenParams {
    pub token_idx: u32,
    pub address: [u8; 20],
}

type UpdateTokenParams = AddTokenParams;

#[derive(Debug, Clone)]
pub enum Data {
    AddMarket(AddMarketParams),
    CloseMarket(CloseMarketParams),
    AddToken(AddTokenParams),
    UpdateToken(UpdateTokenParams),
    RegisterPlayer,
    AddLimitOrder(AddLimitOrderParams),
    AddMarketOrder(AddMarketOrderParams),
    CancelOrder(CancelOrderParams),
    AddTrade(AddTradeParams),
    Transfer(TransferParams),
    Deposit(DepositParams),
    Withdraw(WithdrawParams),
}

#[derive(Debug, Clone)]
pub struct Market {
    /// market id 对应交易对
    pub market_id: u64,
    /// open / close
    pub status: u64,
    /// A token的 token idx 比如是usdt
    pub token_a: u32,
    /// B token 的token idx 比如是eth
    pub token_b: u32,

    pub last_deal_price: u64,
    // todo add market price
}

// 可以为status定义常量
pub const MARKET_STATUS_OPEN: u64 = 1;
pub const MARKET_STATUS_CLOSE: u64 = 0;


impl Market {
    pub fn new(market_id: u64, token_a: u32, token_b: u32, price: u64) -> Self {
        Self {
            market_id,
            status: MARKET_STATUS_OPEN, // 默认为open状态
            token_a,
            token_b,
            last_deal_price: price,
        }
    }

    pub fn get_key(market_id: u64) -> [u64; 4] {
        [0, market_id, COMMON_PREFIX, MARKET_STORE_PREFIX]
    }

    pub fn add_event(&self) {
        let events = unsafe { get_event() };
        events.push(EVENT_MARKET << 32);
        let pos = events.len();
        events.push(self.market_id);
        events.push(self.status);
        events.push(self.token_a as u64);
        events.push(self.token_b as u64);
        events.push(self.last_deal_price);
        events[pos - 1] |= (events.len() - pos) as u64;
    }

    pub fn store(&self) {
        zkwasm_rust_sdk::dbg!("store market\n");
        let mut data = Vec::new();
        self.to_data(&mut data);
        let kvpair = unsafe { &mut MERKLE_MAP };
        kvpair.set(&Self::get_key(self.market_id), data.as_slice());
        zkwasm_rust_sdk::dbg!("end store token\n");
    }

    pub fn load(market_id: u64) -> Option<Self> {
        let kvpair = unsafe { &mut MERKLE_MAP };
        let mut data = kvpair.get(&Self::get_key(market_id));
        if data.is_empty() {
            return None;
        }
        let mut u64data = data.iter_mut();
        Some(Self::from_data(&mut u64data))
    }

    pub fn is_close(&self) -> bool {
        self.status == MARKET_STATUS_CLOSE
    }
}

impl StorageData for Market {
    fn from_data(u64data: &mut IterMut<u64>) -> Self {
        let market_id = *u64data.next().unwrap();
        let status = *u64data.next().unwrap();
        let token_a = *u64data.next().unwrap() as u32;
        let token_b = *u64data.next().unwrap() as u32;
        let last_deal_price = *u64data.next().unwrap();
        Market {
            market_id,
            status,
            token_a,
            token_b,
            last_deal_price,
        }
    }

    fn to_data(&self, data: &mut Vec<u64>) {
        data.push(self.market_id);
        data.push(self.status);
        data.push(self.token_a as u64);
        data.push(self.token_b as u64);
        data.push(self.last_deal_price);
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    /// token idx
    pub token_idx: u32,
    pub address: [u8; 20], // 20
}

impl Token {
    pub fn new(token_idx: u32, address: [u8; 20]) -> Self {
        Self { token_idx, address }
    }

    pub fn get_key(token_idx: u32) -> [u64; 4] {
        [0, token_idx as u64, COMMON_PREFIX, TOKEN_STORE_PREFIX]
    }

    pub fn add_event(&self) {
       let events = unsafe { get_event() };
        events.push(EVENT_TOKEN << 32);
        let pos = events.len();
        events.push(self.token_idx as u64);
        for i in 0..20 {
            events.push(self.address[i] as u64);
        }
        events[pos - 1] |= (events.len() - pos) as u64;
    }

    pub fn store(&self) {
        zkwasm_rust_sdk::dbg!("store token\n");
        let mut data = Vec::new();
        self.to_data(&mut data);
        let kvpair = unsafe { &mut MERKLE_MAP };
        kvpair.set(&Self::get_key(self.token_idx), data.as_slice());
        zkwasm_rust_sdk::dbg!("end store token\n");
    }

    pub fn load(token_idx: u32) -> Option<Self> {
        zkwasm_rust_sdk::dbg!("load token\n");
        let kvpair = unsafe { &mut MERKLE_MAP };
        let mut data = kvpair.get(&Self::get_key(token_idx));
        if data.is_empty() {
            return None;
        }
        let mut u64data = data.iter_mut();
        zkwasm_rust_sdk::dbg!("end token\n");
        Some(Self::from_data(&mut u64data))
    }
}

impl StorageData for Token {
    fn from_data(u64data: &mut IterMut<u64>) -> Self {
        let token_idx = *u64data.next().unwrap() as u32;
        let mut address = [0u8; 20];
        let data = &[
            *u64data.next().unwrap(),
            *u64data.next().unwrap(),
            *u64data.next().unwrap(),
        ];
        for i in 0..20 {
            address[i] = (data[i / 8] >> (i % 8)) as u8;
        }
        Token { token_idx, address }
    }

    fn to_data(&self, data: &mut Vec<u64>) {
        data.push(self.token_idx as u64);
        let mut address = [0u64; 3];
        for i in 0..20 {
            address[i / 8] |= (self.address[i] as u64) << (i % 8);
        }
        data.push(address[0]);
        data.push(address[1]);
        data.push(address[2]);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Order {
    /// 唯一id
    pub id: u64,

    /// order类型 (limit or market order)
    pub type_: u8,

    /// order状态 (live/match/partial_match/partial_cancel/cancel)
    pub status: u8,

    /// player id
    pub pid: [u64; 2],

    /// 表示是哪个交易对的order
    pub market_id: u64,

    /// 第0bit表示buy/sell
    pub flag: u8,

    /// 锁定的balance
    pub lock_balance: u64,

    /// 锁定的手续费(usdt)
    pub lock_fee: u64,

    /// limit order 的限价
    /// 买的时候，是买一个b token的价格
    /// 卖的时候，是卖一个 b token的价格
    pub price: u64,

    /// 数量
    pub b_token_amount: u64,

    pub a_token_amount: u64,

    /// 部分成交的数量
    /// 限价买单: 已经买了多少b token
    /// 市价买单: 已经花了多少a token
    /// 限价卖单: 已经卖了多少b token
    /// 市价卖单: 已经卖多少b token
    pub already_deal_amount: u64,
}

impl Order {
    // Order types
    pub const TYPE_LIMIT: u8 = 0;
    pub const TYPE_MARKET: u8 = 1;

    // Order status
    pub const STATUS_LIVE: u8 = 0;
    pub const STATUS_MATCH: u8 = 1;
    pub const STATUS_PARTIAL_MATCH: u8 = 2;
    pub const STATUS_PARTIAL_CANCEL: u8 = 3;
    pub const STATUS_CANCEL: u8 = 4;

    // Order flags
    pub const FLAG_SELL: u8 = 0;
    pub const FLAG_BUY: u8 = 1;

    pub fn new(
        id: u64,
        type_: u8,
        status: u8,
        pid: [u64; 2],
        market_id: u64,
        flag: u8,
        lock_balance: u64,
        lock_fee: u64,
        price: u64,
        b_token_amount: u64,
        a_token_amount: u64,
        already_deal_amount: u64,
    ) -> Self {
        Self {
            id,
            type_,
            status,
            pid,
            market_id,
            flag,
            lock_balance,
            lock_fee,
            price,
            b_token_amount,
            a_token_amount,
            already_deal_amount,
        }
    }

    pub fn get_key(id: u64) -> [u64; 4] {
        [0, id, ORDER_PREFIX, 2]
    }

    pub fn is_live(&self) -> bool {
        self.status == Order::STATUS_LIVE || self.status == Order::STATUS_PARTIAL_MATCH
    }

    pub fn is_limit_order(&self) -> bool {
        self.type_ == Order::TYPE_LIMIT
    }

    pub fn is_market_order(&self) -> bool {
        self.type_ == Order::TYPE_MARKET
    }

    pub fn is_buy(&self) -> bool {
        self.flag == Order::FLAG_BUY
    }

    pub fn is_sell(&self) -> bool {
        self.flag == Order::FLAG_SELL
    }

    pub fn update_status(&mut self) {
        if self.status == Order::STATUS_CANCEL || self.status == Order::STATUS_PARTIAL_CANCEL {
            return;
        }
        if self.b_token_amount == self.already_deal_amount {
            self.status = Order::STATUS_MATCH;
        } else {
            self.status = Order::STATUS_PARTIAL_MATCH;
        }
    }

    pub fn add_event(&self) {
        let events = unsafe { get_event() };
        events.push(EVENT_ORDER << 32);
        let pos = events.len();
        events.push(self.id);
        events.push(self.type_ as u64);
        events.push(self.status as u64);
        events.push(self.pid[0]);
        events.push(self.pid[1]);
        events.push(self.market_id);
        events.push(self.flag as u64);
        events.push(self.lock_balance);
        events.push(self.lock_fee);
        events.push(self.price);
        events.push(self.b_token_amount);
        events.push(self.a_token_amount);
        events.push(self.already_deal_amount);
        events[pos - 1] |= (events.len() - pos) as u64;
    }

    pub fn store(&self) {
        zkwasm_rust_sdk::dbg!("store order\n");
        let mut data = Vec::new();
        self.to_data(&mut data);
        let kvpair = unsafe { &mut MERKLE_MAP };
        kvpair.set(&Self::get_key(self.id), data.as_slice());
        zkwasm_rust_sdk::dbg!("end store order\n");
    }

    pub fn load(id: u64) -> Option<Self> {
        zkwasm_rust_sdk::dbg!("load order\n");
        let kvpair = unsafe { &mut MERKLE_MAP };
        let mut data = kvpair.get(&Self::get_key(id));
        if data.is_empty() {
            return None;
        }
        let mut u64data = data.iter_mut();
        Some(Self::from_data(&mut u64data))
    }
}

impl StorageData for Order {
    fn from_data(u64data: &mut IterMut<u64>) -> Self {
        let id = *u64data.next().unwrap();
        let type_ = *u64data.next().unwrap() as u8;
        let status = *u64data.next().unwrap() as u8;
        let pid = [*u64data.next().unwrap(), *u64data.next().unwrap()];
        let market_id = *u64data.next().unwrap();
        let flag = *u64data.next().unwrap() as u8;
        let lock_balance = *u64data.next().unwrap();
        let lock_fee = *u64data.next().unwrap();
        let price = *u64data.next().unwrap();
        let b_token_amount = *u64data.next().unwrap();
        let a_token_amount = *u64data.next().unwrap();
        let already_deal_amount = *u64data.next().unwrap();
        Order {
            id,
            type_,
            status,
            pid,
            market_id,
            flag,
            lock_balance,
            lock_fee,
            price,
            b_token_amount,
            a_token_amount,
            already_deal_amount,
        }
    }

    fn to_data(&self, data: &mut Vec<u64>) {
        data.push(self.id);
        data.push(self.type_ as u64);
        data.push(self.status as u64);
        data.push(self.pid[0]);
        data.push(self.pid[1]);
        data.push(self.market_id);
        data.push(self.flag as u64);
        data.push(self.lock_balance);
        data.push(self.lock_fee);
        data.push(self.price);
        data.push(self.b_token_amount);
        data.push(self.a_token_amount);
        data.push(self.already_deal_amount);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Trade {
    /// 唯一id
    pub trade_id: u64,

    /// 买的order id
    pub a_order_id: u64,

    /// 卖的order id
    pub b_order_id: u64,

    /// a的实际数量
    pub a_actual_amount: u64,

    /// b的实际数量
    pub b_actual_amount: u64,
}

impl Trade {
    pub fn new(
        trade_id: u64,
        a_order_id: u64,
        b_order_id: u64,
        a_actual_amount: u64,
        b_actual_amount: u64,
    ) -> Self {
        Self {
            trade_id,
            a_order_id,
            b_order_id,
            a_actual_amount,
            b_actual_amount,
        }
    }

    pub fn get_key(trade_id: u64) -> [u64; 4] {
        [0, trade_id, TRADE_PREFIX, 0]
    }

    pub fn add_event(&self, market_id: u64) {
        let events = unsafe { get_event() };
        events.push(EVENT_TRADE << 32);
        let pos = events.len();
        events.push(self.trade_id);
        events.push(market_id);
        events.push(self.a_order_id);
        events.push(self.b_order_id);
        events.push(self.a_actual_amount);
        events.push(self.b_actual_amount);
        events[pos - 1] |= (events.len() - pos) as u64;
    }

    pub fn store(&self) {
        zkwasm_rust_sdk::dbg!("store trade\n");
        let mut data = Vec::new();
        self.to_data(&mut data);
        let kvpair = unsafe { &mut MERKLE_MAP };
        kvpair.set(&Self::get_key(self.trade_id), data.as_slice());
        zkwasm_rust_sdk::dbg!("end store trade\n");
    }

    pub fn load(trade_id: u64) -> Option<Self> {
        zkwasm_rust_sdk::dbg!("load trade\n");
        let kvpair = unsafe { &mut MERKLE_MAP };
        let mut data = kvpair.get(&Self::get_key(trade_id));
        if data.is_empty() {
            return None;
        }
        let mut u64data = data.iter_mut();
        Some(Self::from_data(&mut u64data))
    }
}

impl StorageData for Trade {
    fn from_data(u64data: &mut IterMut<u64>) -> Self {
        let trade_id = *u64data.next().unwrap();
        let a_order_id = *u64data.next().unwrap();
        let b_order_id = *u64data.next().unwrap();
        let a_actual_amount = *u64data.next().unwrap();
        let b_actual_amount = *u64data.next().unwrap();
        Trade {
            trade_id,
            a_order_id,
            b_order_id,
            a_actual_amount,
            b_actual_amount,
        }
    }

    fn to_data(&self, data: &mut Vec<u64>) {
        data.push(self.trade_id);
        data.push(self.a_order_id);
        data.push(self.b_order_id);
        data.push(self.a_actual_amount);
        data.push(self.b_actual_amount);
    }
}

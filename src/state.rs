use crate::player::{ExchangePlayer};
use crate::settlement::SettlementInfo;
use serde::Serialize;
use zkwasm_rest_abi::{MERKLE_MAP, StorageData};
use crate::transaction::{Order, Trade};

#[derive(Serialize)]
pub struct State {
    pub counter: u64,
    pub total_fee: u64,
    pub market_id_counter: u64,
    pub order_id_counter: u64,
    pub trade_id_counter: u64,
    pub event_id_counter: u64,
    // add debug order, trade, market info
}

const DEBUG: bool = true;

#[derive(Serialize)]
pub struct StateDebug {
    pub counter: u64,
    pub total_fee: u64,
    pub market_id_counter: u64,
    pub order_id_counter: u64,
    pub trade_id_counter: u64,
    pub event_id_counter: u64,
    // add debug order, trade, market info
    pub orders: Vec<Order>,
    pub trades: Vec<Trade>,
}

const MAX_TOKEN_IDX: u32 = 4;

impl State {
    pub fn get_state(pkey: Vec<u64>) -> String {
        let player = ExchangePlayer::get_from_pid(&ExchangePlayer::pkey_to_pid(
            &pkey.try_into().unwrap(),
        ));
        if player.is_none() {
            return  serde_json::to_string(&player).unwrap();
        }
        let mut player = player.unwrap();
        if DEBUG {
            for i in 0..MAX_TOKEN_IDX {
                player.data.load_position(i, &player.player_id);
            }
        }
        serde_json::to_string(&player).unwrap()
    }

    pub fn rand_seed() -> u64 {
        0
    }

    pub fn store() {
        let mut data = Vec::new();
        unsafe { STATE.to_data(&mut data) };
        let kvpair = unsafe { &mut MERKLE_MAP };
        kvpair.set(&Self::get_key(), data.as_slice());
    }

    pub fn initialize() {
        Self::load();
    }

    pub fn new() -> Self {
        State {
            counter: 0,
            total_fee: 0,
            market_id_counter: 0,
            order_id_counter: 0,
            trade_id_counter: 0,
            event_id_counter: 0,
        }
    }

    pub fn snapshot() -> String {
        let state = unsafe { &STATE };
        if !DEBUG {
            return serde_json::to_string(state).unwrap();
        }
        let mut state_debug = StateDebug {
            counter: state.counter,
            total_fee: state.total_fee,
            market_id_counter: state.market_id_counter,
            order_id_counter: state.order_id_counter,
            trade_id_counter: state.trade_id_counter,
            event_id_counter: state.event_id_counter,
            orders: vec![],
            trades: vec![],
        };

        for i in  1..(state.order_id_counter + 1) {
            let order = Order::load(i);
            if order.is_some() {
                state_debug.orders.push(order.unwrap());
            }
        }

        for i in  1..(state.trade_id_counter + 1) {
            let trade = Trade::load(i);
            if trade.is_some() {
                state_debug.trades.push(trade.unwrap());
            }
        }

        serde_json::to_string(&state_debug).unwrap()
    }

    pub fn preempt() -> bool {
        let state = unsafe { &STATE };
        zkwasm_rust_sdk::dbg!("preempt number is {}\n", {state.counter});
        return (state.counter + 1) % 5 == 0;
    }

    pub fn flush_settlement() -> Vec<u8> {
        let data = SettlementInfo::flush_settlement();
        // unsafe {STATE.store()};
        data
    }

    pub fn tick(&mut self) {
        self.counter += 1;
    }

    pub fn get_new_market_id(&mut self) -> u64 {
        self.market_id_counter += 1;
        self.market_id_counter
    }

    pub fn get_new_order_id(&mut self) -> u64 {
        self.order_id_counter += 1;
        self.order_id_counter
    }

    pub fn get_new_trade_id(&mut self) -> u64 {
        self.trade_id_counter += 1;
        let tid = self.trade_id_counter;
        zkwasm_rust_sdk::dbg!("get_new_trade_id {}\n", tid);
        self.trade_id_counter
    }

    pub fn get_new_event_id(&mut self) -> u64 {
        self.event_id_counter += 1;
        self.event_id_counter
    }

    pub fn get_key() -> [u64; 4] {
        [0, 0, 0, 0]
    }

    pub fn load() {
        let kvpair = unsafe { &mut MERKLE_MAP };
        let mut data = kvpair.get(&Self::get_key());
        if !data.is_empty() {
            let mut u64data = data.iter_mut();
            unsafe { STATE = State::from_data(&mut u64data) };
        }
    }


}

impl StorageData for State {
    fn from_data(u64data: &mut std::slice::IterMut<u64>) -> Self {
        State {
            counter: *u64data.next().unwrap(),
            total_fee: *u64data.next().unwrap(),
            market_id_counter: *u64data.next().unwrap(),
            order_id_counter: *u64data.next().unwrap(),
            trade_id_counter: *u64data.next().unwrap(),
            event_id_counter: *u64data.next().unwrap(),
        }
    }

    fn to_data(&self, data: &mut Vec<u64>) {
        data.push(self.counter);
        data.push(self.total_fee);
        data.push(self.market_id_counter);
        data.push(self.order_id_counter);
        data.push(self.trade_id_counter);
        data.push(self.event_id_counter);
    }
}

pub static mut STATE: State = State {
    counter: 0,
    total_fee: 0,
    market_id_counter: 0,
    order_id_counter: 0,
    trade_id_counter: 0,
    event_id_counter: 0,
};

import { Service } from "zkwasm-ts-server";
import {TxWitness} from "zkwasm-ts-server/src/prover";
import {Event, EventModel} from "./info/event.js";
import {Position, TokenModel, Token, PositionModel, Market, MarketModel} from "./info/info.js";
import {Order, OrderModel, Trade, TradeModel, MatchingSystem} from "./matcher/matcher.js";
import { Player} from "./api.js";
import {get_server_admin_key} from "zkwasm-ts-server/src/config.js";
import { Express } from "express";
import {clearTxFromCommit, CommitModel, getTxFromCommit, insertTxIntoCommit} from "./commits.js";
import {merkleRootToBeHexString} from "zkwasm-ts-server/src/lib.js";

const uncommittedTxs: TxWitness[] = [];


const service = new Service(eventCallback, batchedCallback, extra, bootstrap);
await service.initialize();


let currentUncommitMerkleRoot: string = merkleRootToBeHexString(service.merkleRoot);

const msM= new Map<bigint, MatchingSystem>();
// load all market
let markets = await MarketModel.find({});
for(let market of markets) {
  console.log("add market", market.marketId);
  let ms = new MatchingSystem(BigInt(market.marketId));
  msM.set(BigInt(market.marketId), ms);

  let orders = await OrderModel.find({
    market_id: BigInt(market.marketId),
    $or: [{ status: 0 }, { status: 2 }]
  });

  for (let order of orders) {
    let orderObj = Order.fromMongooseDoc(order);
    console.log("orderObj", orderObj);
    ms.upsertOrder(orderObj);
  }
  console.log("trade", ms.tryMatchOrder());
}

function extra (app: Express) {
  app.get('/data/pair/:id', async(req:any, res) => {
    let id:string = req.params.id;
    console.log(id);
    let query = msM.get(BigInt(id));
    res.status(201).send({
      success: true,
      data: query?.queryInfo(),
    });
  });
  app.get('/data/markets', async(req:any, res) => {
    console.log("query...");
    const value = req.body.msg;
    const ks = [];
    console.log(msM);
    console.log(msM.keys());
    for (const a of msM.keys()) {
      console.log(a);
      ks.push(a.toString());
    }
    res.status(201).send({
      success: true,
      data: ks,
    });
  });

}


service.serve();

const EVENT_POSITION = 1;
const EVENT_TOKEN = 2;
const EVENT_MARKET = 3;
const EVENT_ORDER = 4;
const EVENT_TRADE = 5;

let preemptcounter = 0;

async function bootstrap(merkleRoot: string): Promise<TxWitness[]> {
  const txs = await getTxFromCommit(merkleRoot);
  console.log("tsx in bootstrap:", txs);
  return txs;
}

async function batchedCallback(arg: TxWitness[], preMerkle: string, postMerkle: string) {
  currentUncommitMerkleRoot = postMerkle;
  await clearTxFromCommit(currentUncommitMerkleRoot);
  preemptcounter = 0;
}

async function eventCallback(arg: TxWitness, data: BigUint64Array) {
  insertTxIntoCommit(currentUncommitMerkleRoot, arg, preemptcounter);
  preemptcounter ++;

  //console.log("eventCallback", arg, data);
  if(data[0] != 0n) {
    console.log("non-zero return, tx failed");
    return;
  }
  if(data.length <= 2) {
    console.log("no event data");
    return;
  }

  let event = new Event(data[1], data);
  let doc = new EventModel({
    id: event.id.toString(),
    data: Buffer.from(event.data.buffer)
  });
  let result = await doc.save();
  if (!result) {
    console.log("failed to save event");
    throw new Error("save event to db failed");
  }
  let trades: Trade[] = [];
  let needtryMatchSystems = new Set<bigint>();
  let i = 2; // start pos
  while(i < data.length) {
    let eventType = Number(data[i]>>32n);
    let eventLength = data[i]&((1n<<32n)-1n);
    let eventData = data.slice(i+1, i+1+Number(eventLength));
    //console.log("event", eventType, eventLength, eventData);
    switch(eventType) {
      case EVENT_POSITION:
        {
          console.log("position event");
          let position = Position.fromEvent(eventData);
          let doc = await PositionModel.findOneAndUpdate(
              {pid_1: position.pid_1, pid_2: position.pid_2, token_idx: position.token_idx},
              position.toObject(),
              {upsert: true}
          );
          console.log("save position", position.pid_1, position.pid_2, position.token_idx);
        }
        break;
      case EVENT_TOKEN:
        {
          console.log("token event");
          let token = Token.fromEvent(eventData);
          let doc = await TokenModel.findOneAndUpdate({tokenIdx: token.tokenIdx}, token.toObject(), {upsert: true});
          console.log("save token", doc);
        }
        break;
      case EVENT_MARKET:
        {
          console.log("market event");
          let market = Market.fromEvent(eventData);
          let doc = await MarketModel.findOneAndUpdate({marketId: market.marketId}, market.toObject(), {upsert: true});
          console.log("save market", doc);
	  let marketId = BigInt(market.marketId);

	  if (!msM.has(marketId)) {
	    let ms = new MatchingSystem(marketId);
	    msM.set(marketId, ms);
	  }
        }
        break;
      case EVENT_ORDER:
        {
          let order = Order.fromEvent(eventData);
          console.log("order event, order.id=", order.id);
          let doc = await OrderModel.findOneAndUpdate({id: order.id}, order.toObject(), {upsert: true});
          console.log("save order", order.id);
          if(!msM.has(order.market_id)) {
              console.log("market not found");
              throw new Error("market not found, in match system map");
          }
          let ms = msM.get(order.market_id) as MatchingSystem;
          ms.upsertOrder(order);
          needtryMatchSystems.add(order.market_id);
	  console.log(ms.queryInfo());
        }
        break;
      case EVENT_TRADE:
        {
          console.log("trade event");
          let trade = Trade.fromEvent(eventData);
	  let ms = msM.get(trade.market_id) as MatchingSystem;
	  console.log(trade.market_id, ms.queryInfo());
          let doc = await TradeModel.findOneAndUpdate({trade_id: trade.trade_id}, trade.toObject(), {upsert: true});
          console.log("save trade", trade);
        }
        break;
      default:
        console.log("unknown event");
        break;
    }
    i += 1 + Number(eventLength);
  }

  for(let market_id of needtryMatchSystems) {
    let ms = msM.get(market_id) as MatchingSystem;
    // ms.removeMatchOrder();
    let trades = ms.tryMatchOrder();
    console.log("trades", trades);
    if(trades.length == 0) {
      continue;
    }
    let trade = trades[0];
    let player = new Player(get_server_admin_key(), "http://localhost:3000");
    console.log("add trade", trade.a_order_id, trade.b_order_id, trade.a_actual_amount, trade.b_actual_amount);
    player.addTrace(trade.a_order_id, trade.b_order_id, trade.a_actual_amount, trade.b_actual_amount);
  }
}


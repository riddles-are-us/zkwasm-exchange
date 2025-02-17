import { Player} from "../api.js";
import {get_server_admin_key, get_mongoose_db} from "zkwasm-ts-server/src/config.js";
import {hasSubscribers} from "node:diagnostics_channel";
import {Order,OrderModel} from "../matcher/matcher.js";
import * as fs from 'fs';
import mongoose from 'mongoose';
import dotenv from 'dotenv';

dotenv.config();
const fee = 3n;
await mongoose.connect(get_mongoose_db(), {
});

const db = mongoose.connection;
db.on('error', () => {
	console.error('fatal: mongoose connection error ... process will terminate');
	process.exit(1);
});
db.once('open', () => {
	console.log('Connected to MongoDB');
});


function readAccountsFromFile(filePath: string) {
	const jsonData = fs.readFileSync(filePath, 'utf-8');
	return JSON.parse(jsonData);
}
const filename = `randomAccounts.json`;
const accounts = readAccountsFromFile(filename);
const playerA = new Player(accounts[0].account, "http://localhost:3000");
const playerB = new Player(accounts[1].account, "http://localhost:3000");

async function test() {
  let name = "limit order sell and market order buy test, b token amount:";
  console.log(name, "add limit order, sell");
  let state = await playerA.addLimitOrder(1n, 0n, BigInt(1e9), 33n);
  let s_id = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
  console.log("order id=", s_id);

  console.log(name, "add market order, buy");
  state = await playerB.addMarketOrder(1n, 1n, 0n, 33n);
  let b_id = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
  console.log("order id=", b_id, typeof(b_id));

  await Promise.all([
	  wait_for_completed(s_id),
	  wait_for_completed(b_id)
  ]);
  console.log("all order completed");
}

async function s_test(nr:any) {
  let name = "limit order sell and market order buy test, b token amount:";
  console.log(name, "add limit order, sell");
  let orders = [];

  for (let i=0; i<nr; i++){
  let state = await playerA.addLimitOrder(1n, 0n, BigInt(1e9), 33n);
  let s_id = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
  orders.push(s_id);

  state = await playerB.addMarketOrder(1n, 1n, 0n, 33n);
  let b_id = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
  orders.push(b_id);
  }

  let promises = orders.map(s_id => wait_for_completed(s_id));
  await Promise.all(promises);
  console.log("all order completed");
}


async function wait_for_completed(s_id:string) {
	do{
		let order = await OrderModel.findOne({
			id: BigInt(s_id)
		});
		let orderObj = order ? Order.fromMongooseDoc(order) : null; 
		if (orderObj) {
			//console.log("(after)   orderObj", s_id, orderObj);
			if (orderObj.status == Order.STATUS_MATCH || orderObj.status == Order.STATUS_PARTIAL_MATCH) {
				console.log(s_id, "completed with status:", orderObj.status); 
				break;
			}
		}
		await new Promise(resolve => setTimeout(resolve, 1000)); // 1-second delay	
	} while (true);
}

function compareState(before:any, after:any, player:any) {
	console.log("before positon", before.player.data.positions);
	console.log("after positon", after.player.data.positions);
	for (let tokenIdx=0; tokenIdx < before.player.data.positions.length; tokenIdx++){
	 console.log(after.player.data.positions[tokenIdx].balance -before.player.data.positions[tokenIdx].balance);
	}
}

async function main(runTest:any) {
	let playerAStateBefore = await playerA.getState();
	let playerBStateBefore = await playerB.getState();
	if (runTest === undefined || isNaN(runTest))
		await test();
	else 
		await s_test(runTest);
	let playerAStateAfter = await playerA.getState();
	let playerBStateAfter = await playerB.getState();
	compareState(playerAStateBefore, playerAStateAfter,playerA);
	compareState(playerBStateBefore, playerBStateAfter,playerB);

}
const runTest = parseInt(process.argv[2], 10);

await main(runTest);
process.exit(0);


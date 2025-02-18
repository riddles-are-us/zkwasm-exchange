// testPriceSpreadWithComparison.ts
import { Player } from "../api.js";
import {get_server_admin_key, get_mongoose_db} from "zkwasm-ts-server/src/config.js";
import {query} from "zkwasm-ts-server";
import {Order,OrderModel, PRICISION} from "../matcher/matcher.js";
import mongoose from 'mongoose';
import dotenv from 'dotenv';
import {PrivateKey, bnToHexLe} from "delphinus-curves/src/altjubjub";

// Constant fee defined in your system
const fee = 3n;
const SELL = 0n;
const BUY = 1n;
let id = "0";
const nrMaker = 3;

function getPkey(player:Player) : string{ 
let pkey = PrivateKey.fromString(player.processingKey);
let pubkey = pkey.publicKey.key.x.v;
return bnToHexLe(pubkey);
}

dotenv.config();
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

async function waitForOrderCompletion(s_id:string) {
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


async function simulationTestPriceSpreadWithComparison() {
  const makerPrices = [990n*PRICISION, 1000n*PRICISION, 1010n*PRICISION];
  const makerAmounts = [50n, 60n, 70n];
  const totalAmount = makerAmounts.reduce((a, b) => a + b, 0n);
  const takerPrices = 1020n*PRICISION;

  const makers: Player[] = [];
  for (let i = 0; i < nrMaker; i++) {
    makers.push(new Player(`00012345${i}`, "http://localhost:3000"));
  }
  const taker = new Player("11112345", "http://localhost:3000");

  const admin = new Player(get_server_admin_key(), "http://localhost:3000");
  console.log("Admin registering...");
  await admin.register();

  console.log("Admin adding tokens and market for price spread test...");
  await admin.addToken(0n,"0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
  await admin.addToken(1n, "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
  await admin.addToken(2n, "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
  await admin.addMarket(1n, 2n, 100n);

  for (let i = 0; i < nrMaker; i++) {
    console.log(`Maker ${i}: registering and depositing tokenB and tokenA...`);
    await makers[i].register();
    await admin.deposit(getPkey(makers[i]), 0n, 100000n);
    await admin.deposit(getPkey(makers[i]), 1n, 100000n);
    await admin.deposit(getPkey(makers[i]), 2n, 100000n);
  //  await makers[i].deposit("428c73246352807b9b31b84ff788103abc7932b72801a1b23734e7915cc7f610",0n, 100000n);
  }

  console.log("Taker: registering and depositing tokenA and tokenB...");
  await taker.register();
  await admin.deposit(getPkey(taker), 0n, 1000000n);
  await admin.deposit(getPkey(taker), 1n, 1000000n);
  await admin.deposit(getPkey(taker), 2n, 1000000n);

  // Step 2: Record initial balances
  const initialBalances = {
    taker: await taker.getState(),
    makers: await Promise.all(makers.map(m => m.getState()))
  };

  // Step 3: Place orders
  let makerOrderIds = [];
  for (let i = 0; i < nrMaker; i++) {
    const state = await makers[i].addLimitOrder(1n, SELL, makerPrices[i], makerAmounts[i]);
    id = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
    makerOrderIds.push(id);
    console.log(`Maker ${i} placing limit sell order... ${id}`, "amount:", makerAmounts[i]);
  }

  const state = await taker.addLimitOrder(1n, BUY, takerPrices, totalAmount);
  const takerOrderId = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
  console.log(`Taker placing limit buy order...${takerOrderId}`, "amount:", totalAmount);

  // Step 4: Wait for orders to complete
  await waitForOrderCompletion(takerOrderId);
  for (let i = 0; i < nrMaker; i++) {
    await waitForOrderCompletion(makerOrderIds[i]);
  }

  // Step 5: Fetch final balances
  const finalBalances = {
    taker: await taker.getState(),
    makers: await Promise.all(makers.map(m => m.getState()))
  };

  // Step 6: Compare expected vs actual balance changes
  console.log("Comparing balances...");

  let expectedTakerATokenSpent = 0n;
  let expectedTakerBTokenReceived = 0n;

  for (let i = 0; i < nrMaker; i++) {
    const expectedMakerAReceived = makerAmounts[i] * (makerPrices[i]/PRICISION);
    const expectedMakerBSpent = makerAmounts[i];

    const makerInitial = initialBalances.makers[i].player.data.positions;
    const makerFinal = finalBalances.makers[i].player.data.positions;

    const actualMakerAReceived = BigInt(makerFinal[1].balance - makerInitial[1].balance);
    const actualMakerBSpent = BigInt(makerInitial[2].balance - makerFinal[2].balance);

    console.log(makerInitial);
    console.log(makerFinal);
    if (actualMakerAReceived !== expectedMakerAReceived) {
      console.

error(`Maker ${i} tokenA mismatch! Expected ${expectedMakerAReceived}, got ${actualMakerAReceived}`);
    }
    if (actualMakerBSpent !== expectedMakerBSpent) {
      console.error(`Maker ${i} tokenB mismatch! Expected ${expectedMakerBSpent}, got ${actualMakerBSpent}`);
    }

    expectedTakerATokenSpent += expectedMakerAReceived;
    expectedTakerBTokenReceived += makerAmounts[i];
  }

  const takerInitial = initialBalances.taker.player.data.positions;
  const takerFinal = finalBalances.taker.player.data.positions;

  const actualTakerASpent = BigInt(takerInitial[1].balance - takerFinal[1].balance);
  const actualTakerBReceived = BigInt(takerFinal[2].balance - takerInitial[2].balance);

  if (actualTakerASpent !== expectedTakerATokenSpent) {
    console.error(`Taker tokenA mismatch! Expected ${expectedTakerATokenSpent}, got ${actualTakerASpent}`);
  }
  if (actualTakerBReceived !== expectedTakerBTokenReceived) {
    console.error(`Taker tokenB mismatch! Expected ${expectedTakerBTokenReceived}, got ${actualTakerBReceived}`);
  }
    console.log(takerInitial);
    console.log(takerFinal);

  console.log("Price spread test with balance comparison completed.");
}

simulationTestPriceSpreadWithComparison().catch((err) => {
  console.error("Test failed:", err);
});

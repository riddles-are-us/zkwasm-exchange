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
const nrMaker = 8;
const nrTaker = 4;

let makerPrices : bigint[] = [];
let makerAmounts : bigint[] = [];
for (let i = 0; i < nrMaker; i++) {
	  makerPrices.push((10n + BigInt(i * 10)) * PRICISION);  // Adjust price
	  makerAmounts.push(16n + BigInt(i * 10));  // Adjust amount
}
let totalMakerValue = makerPrices.reduce((sum, price, index) => sum + (price * makerAmounts[index]), 0n);
let totalMakerAmount = makerAmounts.reduce((sum, amount) => sum + amount, 0n);

console.log(totalMakerValue);
console.log(makerPrices, makerAmounts);

let totalTakerAmount = 0n;
let equalValuePerTaker = totalMakerAmount / BigInt(nrTaker);

let takerPrices : bigint[] = [];
let takerAmounts : bigint[] = [];
for (let i = 0; i < nrTaker; i++) {
	 let increasedTakerPrice = makerPrices[nrMaker-1] * (105n+BigInt(i*10))/100n;//+5%;
         takerPrices.push(increasedTakerPrice);

	 let takerAmount = equalValuePerTaker;
	 takerAmounts.push(takerAmount);
	          
	 // Keep track of total amount to balance it
	 totalTakerAmount += takerAmount;
}
console.log(totalTakerAmount);
console.log(takerPrices, takerAmounts);

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
  const makers: Player[] = [];
  for (let i = 0; i < nrMaker; i++) {
    makers.push(new Player(`00012345${i}`, "http://localhost:3000"));
  }
  const takers: Player[] = [];
  for (let i = 0; i < nrTaker; i++) {
    takers.push(new Player(`11112345${i}`, "http://localhost:3000"));
  }

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
  }

  for (let i = 0; i < nrTaker; i++) {
    console.log(`Taker ${i}: registering and depositing tokenA and tokenB...`);
    await takers[i].register();
    await admin.deposit(getPkey(takers[i]), 0n, 1000000n);
    await admin.deposit(getPkey(takers[i]), 1n, 1000000n);
    await admin.deposit(getPkey(takers[i]), 2n, 1000000n);
  }

  // Step 2: Record initial balances
  const initialBalances = {
    takers: await Promise.all(takers.map(m=> m.getState())),
    makers: await Promise.all(makers.map(m => m.getState()))
  };

  // Step 3: Place orders
  let makerOrderIds: { [key: number]: string[] } = {};
  for (let i = 0; i < nrMaker; i++) {
    if (!makerOrderIds[i])
        makerOrderIds[i] = []; // Initialize the array for taker[i]

    let state = await makers[i].addLimitOrder(1n, SELL, makerPrices[i], makerAmounts[i]);
    id = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
    makerOrderIds[i].push(id);

    state = await makers[i].addLimitOrder(1n, SELL, makerPrices[i], makerAmounts[i]);
    id = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
    makerOrderIds[i].push(id);
    console.log(`Maker ${i} placing limit sell order ${id}, ${makerOrderIds[i]}`, "amount:", makerAmounts[i]);
  }

  let takerOrderIds: { [key: number]: string[] } = {};
  for (let i = 0; i < nrTaker; i++) {
    if (!takerOrderIds[i])
        takerOrderIds[i] = []; // Initialize the array for taker[i]
    let state = await takers[i].addLimitOrder(1n, BUY, takerPrices[i], takerAmounts[i]);
    id = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
    takerOrderIds[i].push(id);

    state = await takers[i].addLimitOrder(1n, BUY, takerPrices[i], takerAmounts[i]);
    id = JSON.stringify(state.state.orders[state.state.orders.length-1].id, null, 3);
    takerOrderIds[i].push(id);
    console.log(`Taker ${i} placing limit buy order ${id}, ${takerOrderIds[i]}`, "amount:", takerAmounts[i]);
  }

  // Step 4: Wait for orders to complete
  for (let i = 0; i < nrTaker; i++) {
	  const orderIds = takerOrderIds[i];
	  await Promise.all(orderIds.map(orderId => waitForOrderCompletion(orderId)))
  }
  for (let i = 0; i < nrMaker; i++) {
	  const orderIds = makerOrderIds[i];
	  await Promise.all(orderIds.map(orderId => waitForOrderCompletion(orderId)))
  }

  // Step 5: Fetch final balances
  const finalBalances = {
    takers: await Promise.all(takers.map(m=> m.getState())),
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

error(`Maker ${i} tokenA mismatch! Expected recv ${expectedMakerAReceived}, got ${actualMakerAReceived}`);
    }
    if (actualMakerBSpent !== expectedMakerBSpent) {
      console.error(`Maker ${i} tokenB mismatch! Expected spent ${expectedMakerBSpent}, got ${actualMakerBSpent}`);
    }

  }

  for (let i = 0; i < nrTaker; i++) {
    expectedTakerATokenSpent = takerAmounts[i] * (takerPrices[i]/PRICISION); //tbd:maybe use markerPrices[i] 
    expectedTakerBTokenReceived = takerAmounts[i];

    const takerInitial = initialBalances.takers[i].player.data.positions;
    const takerFinal = finalBalances.takers[i].player.data.positions;

    const actualTakerASpent = BigInt(takerInitial[1].balance - takerFinal[1].balance);
    const actualTakerBReceived = BigInt(takerFinal[2].balance - takerInitial[2].balance);

    if (actualTakerASpent !== expectedTakerATokenSpent) {
      console.error(`Taker tokenA mismatch! Expected spent ${expectedTakerATokenSpent}, acutal spent ${actualTakerASpent}`);
    }
    if (actualTakerBReceived !== expectedTakerBTokenReceived) {
      console.error(`Taker tokenB mismatch! Expected recv ${expectedTakerBTokenReceived}, got ${actualTakerBReceived}`);
    }
    console.log(takerInitial);
    console.log(takerFinal);
  }
  console.log("Price spread test with balance comparison completed.");
}

simulationTestPriceSpreadWithComparison().catch((err) => {
  console.error("Test failed:", err);
});

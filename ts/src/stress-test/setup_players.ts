//import initHostBind, * as hostbind from "./wasmbind/hostbind.js";
//import initHostBind, * as hostbind from "./wasmbind/hostbind.js";
import { Player} from "../api.js";
import {get_server_admin_key} from "zkwasm-ts-server/src/config.js";
import { query} from "zkwasm-ts-server";
import {hasSubscribers} from "node:diagnostics_channel";
import * as fs from 'fs';
import { fee, accountNumber, BUY, SELL, buyAmount, sellAmount } from './consts.js';


let admin_player = new Player(get_server_admin_key(), "http://localhost:3000");


function readAccountsFromFile(filePath: string) {
	const jsonData = fs.readFileSync(filePath, 'utf-8');
	return JSON.parse(jsonData);
}

let state = await admin_player.register();

console.log("add token 0");
state = await admin_player.addToken(0n, "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

console.log("update token 0");
state = await admin_player.updateToken(0n, "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");

console.log("add token 1");
state = await admin_player.addToken(1n, "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");

console.log("add token 2");
state = await admin_player.addToken(2n, "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
console.log("add token market");
state = await admin_player.addMarket(1n, 2n, 100n);
console.log(state);

async function main() {
	try {
		const filename = `randomAccounts.json`;
		const accounts = readAccountsFromFile(filename);

		for (const accountObj of accounts) {
			const account = accountObj.account;
			const address = accountObj.address;  
			const player = new Player(account, "http://localhost:3000");
			let state = await player.register();

			//admin
			let playerPid = query(account).pkx;
			console.log(playerPid);

			console.log("Deposit 300000 tokens  2 to the player");
			state = await admin_player.deposit(playerPid, 2n, 300000n);
			state = await player.getState();
			console.log(JSON.stringify(state, null, 3));
			console.log("Deposit 300000 tokens  1 to the player");
			state = await admin_player.deposit(playerPid, 1n, 300000n);
			state = await player.getState();
			console.log(JSON.stringify(state, null, 3));
			console.log(JSON.stringify(state.player.data.positions, null, 3));
			console.log("Deposit 10000 tokens  0 to the player for fee");
			state = await admin_player.deposit(playerPid, 0n, 10000n);
			state = await player.getState();
			console.log(JSON.stringify(state, null, 3));
                        //console.log(state);
		}

	} catch (e) {
		console.log(e);
	}
}

main();


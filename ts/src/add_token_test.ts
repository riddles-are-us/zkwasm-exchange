//import initHostBind, * as hostbind from "./wasmbind/hostbind.js";
//import initHostBind, * as hostbind from "./wasmbind/hostbind.js";
import { Player} from "./api.js";
import {get_server_admin_key} from "zkwasm-ts-server/src/config.js";
import {hasSubscribers} from "node:diagnostics_channel";
let account = "12345";
let player = new Player(get_server_admin_key(), "http://localhost:3000");
let playerB = new Player(account, "http://localhost:3000");

const fee = 3n;

async function orderCheck(player:Player, f: ()=> void, check:(before: any, after: any) => boolean) {
    let before = await player.getState();
    await f();
    let after = await player.getState();
    if(!check(before, after)) {
        console.log("orderCheck failed");
        throw new Error("orderCheck failed");
    }
}

async function main() {
  //let towerId = 10038n + y;
  let state = await player.getState();
  console.log(state);

  state = await player.register();
  console.log(state);

  state = await playerB.register();
  console.log(state);

  console.log("add token 0");
  state = await player.addToken(0n, "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
  console.log(state);

  console.log("update token 0");
  state = await player.updateToken(0n, "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
  console.log(state);


  console.log("add token 1");
  state = await player.addToken(1n, "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
  console.log(state);

  console.log("add token 2");
  state = await player.addToken(2n, "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
  console.log(state);

  console.log("add token market");
  state = await player.addMarket(1n, 2n, 100n);
  console.log(state);

  console.log("Deposit 10000 tokens 0 to the player");
  state = await player.deposit("428c73246352807b9b31b84ff788103abc7932b72801a1b23734e7915cc7f610", 0n, 300000n);
  console.log(state);

  console.log("Deposit 10000 tokens  1 to the playerB");
  state = await player.deposit("0c839e6ff76723263a6261ce07813303fd2895a07c67369f657fcf6af320292b", 1n, 300000n);
  console.log(state);
  state = await player.deposit("0c839e6ff76723263a6261ce07813303fd2895a07c67369f657fcf6af320292b", 0n, 300000n);
  console.log(state);
  state = await player.deposit("0c839e6ff76723263a6261ce07813303fd2895a07c67369f657fcf6af320292b", 2n, 300000n);
  console.log(state);

  state = await playerB.getState();
  console.log(JSON.stringify(state, null, 3));
}

main();


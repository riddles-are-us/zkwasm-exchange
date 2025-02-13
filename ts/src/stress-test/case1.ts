//import initHostBind, * as hostbind from "./wasmbind/hostbind.js";
//import initHostBind, * as hostbind from "./wasmbind/hostbind.js";
import { Player} from "../api.js";
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
  let name = "limit order buy and market order sell test, b token amount:";
  console.log(name, "add limit order");
  let f = async () => {
    await player.addLimitOrder(1n, 1n, BigInt(1e9), 100n);
  }
  await orderCheck(player, f, (before, after): boolean => {
    let tokenIdx = 0;
    // console.log("before", before);
    // console.log("after", after);
    // console.log("before positon", before.player.data.positions[tokenIdx]);
    // console.log("after positon", after.player.data.positions[tokenIdx]);
    if(("order_id_counter" in before.state?before.state["order_id_counter"]:0) + 1 != ("order_id_counter" in after.state?after.state["order_id_counter"]:0)) {
      return false;
    }
    // @ts-ignore
    if (after.player.data.positions[tokenIdx].lock_balance - before.player.data.positions[tokenIdx].lock_balance != 100n + fee) {
        return false;
    }
    // @ts-ignore
    if (before.player.data.positions[tokenIdx].balance - after.player.data.positions[tokenIdx].balance != 100n + fee) {
      return false;
    }
    return true
  });

  console.log(name, "add market order");
  f = async () => {
    await playerB.addMarketOrder(1n, 0n, 100n, 0n);
  };

  await orderCheck(playerB, f, (before, after): boolean => {
    // console.log("before", before);
    // console.log("after", after);
    // console.log("before positon", before.player.data.positions[tokenIdx]);
    // console.log("after positon", after.player.data.positions[tokenIdx]);
    if(("order_id_counter" in before.state?before.state["order_id_counter"]:0) + 1 != ("order_id_counter" in after.state?after.state["order_id_counter"]:0)) {
      console.log("order_id_counter", before.state["order_id_counter"], after.state["order_id_counter"]);
      return false;
    }
    let tokenIdx = 0;
    // @ts-ignore
    if (after.player.data.positions[tokenIdx].lock_balance - before.player.data.positions[tokenIdx].lock_balance != fee) {
      console.log("fee lock_balance", after.player.data.positions[tokenIdx].lock_balance, before.player.data.positions[tokenIdx].lock_balance);
      return false;
    }
    // @ts-ignore
    if (before.player.data.positions[tokenIdx].balance - after.player.data.positions[tokenIdx].balance != fee) {
      console.log("fee balance", before.player.data.positions[tokenIdx].balance, after.player.data.positions[tokenIdx].balance);
      return false;
    }

    tokenIdx = 1;
    // @ts-ignore
    if (after.player.data.positions[tokenIdx].lock_balance - before.player.data.positions[tokenIdx].lock_balance != 100n) {
      console.log("lock_balance", after.player.data.positions[tokenIdx].lock_balance, before.player.data.positions[tokenIdx].lock_balance);
      return false;
    }
    // @ts-ignore
    if (before.player.data.positions[tokenIdx].balance - after.player.data.positions[tokenIdx].balance != 100n) {
      console.log("balance", before.player.data.positions[tokenIdx].balance, after.player.data.positions[tokenIdx].balance);
      return false;
    }
    return true
  });

  /*
  state = await player.getState();

  console.log(JSON.stringify(state, null, 2));

  console.log(name, "add trade");
  // @ts-ignore
  state = await player.addTrace(BigInt(state.state.order_id_counter - 1), BigInt(state.state.order_id_counter), 100n, 100n);
  //console.log(state);
  console.log(JSON.stringify(state, null, 2));
 */
}

main();


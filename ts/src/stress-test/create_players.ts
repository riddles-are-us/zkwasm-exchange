import * as fs from 'fs';
import { ethers } from 'ethers';

const accountNumber = 20; //create 20 players

async function get_account(priv:any, pub:any) {
	  let provider = new ethers.JsonRpcProvider("https://sepolia.infura.io/v3/4b4be6c5b4bd470f82aef19420785482");
	    const signer = new ethers.Wallet(priv, provider);

	      const l2address = await signer.signMessage(pub);
	        console.log(l2address.substring(2,34));
		  return l2address.substring(2,34);
}


async function createRandomAccount() {
	    const wallet = ethers.Wallet.createRandom();  
	        return {
			        address: wallet.address,
				        privateKey: wallet.privateKey,
						account: await get_account(wallet.privateKey, wallet.address)
						    };
}

const accounts: Array<{ account: string; address: string; privateKey: string }> = [];

for (let i = 0; i < accountNumber; i++) {
	    accounts.push(await createRandomAccount());
}


const jsonData = JSON.stringify(accounts, null, 2);
const filename = `randomAccounts.json`;
fs.writeFileSync(filename, jsonData);

console.log('20 random Ethereum accounts written to ', filename);


import { ApiPromise, WsProvider } from '@polkadot/api';
import { polkadotLocal, WasmProvider } from './polkadot';

import {
  createError, createLog, createWrapper
} from './commons';

const ALICE = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';

// const wsLocal = new WsProvider('ws://127.0.0.1:9944');
// const wsRemote = new WsProvider('wss://poc3-rpc.polkadot.io/');
console.log('polkadotLocal', polkadotLocal)
const wasmLocal = new WasmProvider(polkadotLocal());

(async function main () {
  createLog(`Connecting to Wasm Light Client`);
  try {
    // Create our API with a connection to the Wasm light client 
    const api = await ApiPromise.create(wasmLocal);
    console.log('API created!');
    // Wait until we are ready and connected
    await api.isReady;

    // Do something
    console.log('WASM api', api);
    console.log('api.genesisHash.toHex()', api.genesisHash.toHex());

    // Retrieve the initial data
    let [ , , [data, {free: previous}] ]= await api.query.system.account(ALICE);

    createLog(`Alice has a balance of ${previous}`);

    // Subscribe and listen to balance changes
    api.query.system.account(ALICE, ([, , [data, { free }] ]) => {
      // Calculate the delta
      const change = free.sub(previous);
      // Only display positive value changes (Since we are pulling 'previous' above already,
      // the initial balance change will also be zero)
      if (!change.isZero()) {
        previous = free;
        createLog('New transaction of: '+ change);
      }
    });

  } catch (e) {
    createError(e, wrapper);
  }
}());

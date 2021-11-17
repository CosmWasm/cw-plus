import axios from  "axios";
import fs from "fs";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { GasPrice } from "@cosmjs/stargate";
import { DirectSecp256k1HdWallet, makeCosmoshubPath } from "@cosmjs/proto-signing";
import { HdPath } from "@cosmjs/crypto";
import path from "path";

/*
 * This base-helper is here to use with different helper files in contracts
 *
 * Usage: npx @cosmjs/cli@^0.26 --init https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/base-helpers.ts
 *
 * Create a client:
 *   const [addr, client] = await useOptions(pebblenetOptions).setup('password');
 *
 * Get the mnemonic:
 *   await useOptions(pebblenetOptions).recoverMnemonic(password);
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
*/

interface Options {
  readonly httpUrl: string
  readonly networkId: string
  readonly feeToken: string
  readonly bech32prefix: string
  readonly hdPath: HdPath
  readonly faucetUrl?: string
  readonly defaultKeyFile: string,
  readonly fees: {
    upload: number,
    init: number,
    exec: number
  },
  readonly gasPrice: GasPrice,
}

const pebblenetOptions: Options = {
  httpUrl: 'https://rpc.pebblenet.cosmwasm.com',
  networkId: 'pebblenet-1',
  bech32prefix: 'wasm',
  feeToken: 'upebble',
  faucetUrl: 'https://faucet.pebblenet.cosmwasm.com/credit',
  hdPath: makeCosmoshubPath(0),
  defaultKeyFile: path.join(process.env.HOME, ".pebblenet.key"),
  fees: {
    upload: 1500000,
    init: 500000,
    exec: 200000,
  },
  gasPrice: GasPrice.fromString("0.01upebble"),
}

const uniOptions: Options = {
  httpUrl: 'https://rpc.uni.juno.deuslabs.fi',
  networkId: 'uni',
  bech32prefix: 'juno',
  feeToken: 'ujunox',
  faucetUrl: 'https://faucet.uni.juno.deuslabs.fi/credit',
  hdPath: makeCosmoshubPath(0),
  defaultKeyFile: path.join(process.env.HOME, ".uni.key"),
  fees: {
    upload: 6000000,
    init: 500000,
    exec: 200000,
  },
  gasPrice: GasPrice.fromString("0.025ujunox"),
}

interface Network {
  setup: (password: string, filename?: string) => Promise<[string, SigningCosmWasmClient]>
  recoverMnemonic: (password: string, filename?: string) => Promise<string>
}

const useOptions = (options: Options): Network => {

  const loadOrCreateWallet = async (options: Options, filename: string, password: string): Promise<DirectSecp256k1HdWallet> => {
    let encrypted: string;
    try {
      encrypted = fs.readFileSync(filename, 'utf8');
    } catch (err) {
      // generate if no file exists
      const wallet = await DirectSecp256k1HdWallet.generate(12, {hdPaths: [options.hdPath], prefix: options.bech32prefix});
      const encrypted = await wallet.serialize(password);
      fs.writeFileSync(filename, encrypted, 'utf8');
      return wallet;
    }
    // otherwise, decrypt the file (we cannot put deserialize inside try or it will over-write on a bad password)
    const wallet = await DirectSecp256k1HdWallet.deserialize(encrypted, password);
    return wallet;
  };

  const connect = async (
    wallet: DirectSecp256k1HdWallet,
    options: Options
  ): Promise<SigningCosmWasmClient> => {
    const clientOptions = {
      prefix: options.bech32prefix
    }
    return await SigningCosmWasmClient.connectWithSigner(options.httpUrl, wallet, clientOptions)
  };

  const hitFaucet = async (
    faucetUrl: string,
    address: string,
    denom: string
  ): Promise<void> => {
    await axios.post(faucetUrl, {denom, address});
  }

  const setup = async (password: string, filename?: string): Promise<[string, SigningCosmWasmClient]> => {
    const keyfile = filename || options.defaultKeyFile;
    const wallet = await loadOrCreateWallet(options, keyfile, password);
    const client = await connect(wallet, options);

    const [account] = await wallet.getAccounts();
    // ensure we have some tokens
    if (options.faucetUrl) {
      const tokens = await client.getBalance(account.address, options.feeToken)
      if (tokens.amount === '0') {
        console.log(`Getting ${options.feeToken} from faucet`);
        await hitFaucet(options.faucetUrl, account.address, options.feeToken);
      }
    }

    return [account.address, client];
  }

  const recoverMnemonic = async (password: string, filename?: string): Promise<string> => {
    const keyfile = filename || options.defaultKeyFile;
    const wallet = await loadOrCreateWallet(options, keyfile, password);
    return wallet.mnemonic;
  }

  return {setup, recoverMnemonic};
}

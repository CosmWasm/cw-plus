import axios from  "axios";
import fs from "fs";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { GasPrice, calculateFee, StdFee } from "@cosmjs/stargate";
import { DirectSecp256k1HdWallet, makeCosmoshubPath } from "@cosmjs/proto-signing";
import { Slip10RawIndex } from "@cosmjs/crypto";
import { toUtf8, toBase64 } from "@cosmjs/encoding";
import path from "path";

/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * With these you can easily use the cw20 contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli@^0.26 --init https://raw.githubusercontent.com/CosmWasm/cosmwasm-plus/master/contracts/cw4-group/helpers.ts
 *
 * Create a client:
 *   const [addr, client] = await useOptions(pebblenetOptions).setup('password');
 *
 * Get the mnemonic:
 *   await useOptions(pebblenetOptions).recoverMnemonic(password);
 *
 * Create contract:
 *   const contract = CW4Group(client, pebblenetOptions.fees);
 *
 * Upload contract:
 *   const codeId = await contract.upload(addr);
 *
 * Instantiate contract example:
 *   const initMsg = {
 *     admin: addr,
 *     members: [
 *       {
 *          addr: "wasm1hkxhcvw6sfyu6ztkce3dlz5nnk8kwjmcd7ettt",
 *          weight: 10,
 *       },
 *       {
 *          addr: "wasm1z6ms6cejaj8jz8zwkntx9ua0klhtptvz8elaxp",
 *          weight: 15,
 *       },
 *     ]
 *   };
 *   const instance = await contract.instantiate(addr, codeId, initMsg, 'WORKFORCE1');
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
*/

interface Options {
  readonly httpUrl: string
  readonly networkId: string
  readonly feeToken: string
  readonly bech32prefix: string
  readonly hdPath: readonly Slip10RawIndex[]
  readonly faucetUrl?: string
  readonly defaultKeyFile: string,
  readonly fees: {
    upload: StdFee,
    init: StdFee,
    exec: StdFee
  }
}

const pebblenetGasPrice = GasPrice.fromString("0.01upebble");
const pebblenetOptions: Options = {
  httpUrl: 'https://rpc.pebblenet.cosmwasm.com',
  networkId: 'pebblenet-1',
  bech32prefix: 'wasm',
  feeToken: 'upebble',
  faucetUrl: 'https://faucet.pebblenet.cosmwasm.com/credit',
  hdPath: makeCosmoshubPath(0),
  defaultKeyFile: path.join(process.env.HOME, ".pebblenet.key"),
  fees: {
    upload: calculateFee(1500000, pebblenetGasPrice),
    init: calculateFee(500000, pebblenetGasPrice),
    exec: calculateFee(200000, pebblenetGasPrice),
  },
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
    const wallet = await loadOrCreateWallet(pebblenetOptions, keyfile, password);
    const client = await connect(wallet, pebblenetOptions);

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
    const wallet = await loadOrCreateWallet(pebblenetOptions, keyfile, password);
    return wallet.mnemonic;
  }

  return {setup, recoverMnemonic};
}

interface AdminResponse {
  readonly admin?: string
}

interface MemberResponse {
  readonly weight?: number;
}

interface MemberListResponse {
  readonly members: number;
}

interface TotalWeightResponse {
  readonly weight: number;
}

interface HooksResponse {
  readonly hooks: readonly string[];
}

interface CW4GroupInstance {
  readonly contractAddress: string

  // queries
  admin: () => Promise<AdminResponse>
  totalWeight: () => Promise<TotalWeightResponse>
  member: (addr: string, atHeight?: number) => Promise<MemberResponse>
  listMembers: (startAfter?: string, limit?: number) => Promise<MemberListResponse>
  hooks: () => Promise<HooksResponse>

  // actions
  updateAdmin: (txSigner: string, admin?: string) => Promise<string>
  updateMembers: (txSigner: string, remove: string[], add: string[] ) => Promise<string>
  addHook: (txSigner: string, addr: string) => Promise<string>
  removeHook: (txSigner: string, addr: string) => Promise<string>
}

interface CW4GroupContract {
  upload: (txSigner: string) => Promise<number>
  instantiate: (txSigner: string, codeId: number, initMsg: Record<string, unknown>, label: string, admin?: string) => Promise<CW4GroupInstance>
  use: (contractAddress: string) => CW4GroupInstance
}

export const CW4Group = (client: SigningCosmWasmClient, fees: Options['fees']): CW4GroupContract => {
  const use = (contractAddress: string): CW4GroupInstance => {

    const admin = async (): Promise<AdminResponse> => {
      return client.queryContractSmart(contractAddress, {admin: {}});
    };

    const totalWeight = async (): Promise<TotalWeightResponse> => {
      return client.queryContractSmart(contractAddress, {total_weight: {}});
    };

    const member = async (addr: string, atHeight?: number): Promise<MemberResponse> => {
      return client.queryContractSmart(contractAddress, {member: {addr, at_height: atHeight}});
    };

    const listMembers = async (startAfter?: string, limit?: number): Promise<MemberListResponse> => {
      return client.queryContractSmart(contractAddress, {list_members: {start_after: startAfter, limit}});
    };

    const hooks = async (): Promise<HooksResponse> => {
      return client.queryContractSmart(contractAddress, {hooks: {}});
    };

    const updateAdmin = async (txSigner: string, admin?: string): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {update_admin: {admin}}, fees.exec);
      return result.transactionHash;
    }

    const updateMembers = async (txSigner: string, remove: string[], add: string[]): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {update_members: {remove, add}}, fees.exec);
      return result.transactionHash;
    }

    const addHook = async (txSigner: string, addr: string): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {add_hook: {addr}}, fees.exec);
      return result.transactionHash;
    }

    const removeHook = async (txSigner: string, addr: string): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {remove_hook: {addr}}, fees.exec);
      return result.transactionHash;
    }

    return {
      contractAddress,
      admin,
      totalWeight,
      member,
      listMembers,
      hooks,
      updateAdmin,
      updateMembers,
      addHook,
      removeHook
    };
  }

  const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: 'arraybuffer' })
    if (r.status !== 200) {
      throw new Error(`Download error: ${r.status}`)
    }
    return r.data
  }

  const upload = async (senderAddress: string): Promise<number> => {
    const sourceUrl = "https://github.com/CosmWasm/cosmwasm-plus/releases/download/v0.9.0/cw4_group.wasm";
    const wasm = await downloadWasm(sourceUrl);
    const result = await client.upload(senderAddress, wasm, fees.upload);
    return result.codeId;
  }

  const instantiate = async (senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, admin?: string): Promise<CW4GroupInstance> => {
    const result = await client.instantiate(senderAddress, codeId, initMsg, label, fees.init, { memo: `Init ${label}`, admin });
    return use(result.contractAddress);
  }

  return { upload, instantiate, use };
}

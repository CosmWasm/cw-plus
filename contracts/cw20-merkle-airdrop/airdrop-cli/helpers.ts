import axios from  "axios"
import fs from "fs"
import { SigningCosmWasmClient, CosmWasmFeeTable} from "@cosmjs/cosmwasm-stargate"
import { GasPrice, Secp256k1HdWallet, GasLimits, makeCosmoshubPath } from "@cosmjs/launchpad"
import { Slip10RawIndex } from "@cosmjs/crypto"
import path from "path"
import keccak256 from 'keccak256';
import { MerkleTree } from 'merkletreejs';

/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * With these you can easily use the cw20 contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli@^0.25 --init <add testnet> --init https://raw.githubusercontent.com/CosmWasm/cosmwasm-plus/master/contracts/cw20-merkle-airdrop/airdrop-clip/helpers.ts
 *
 * Create a client:
 *   const [addr, client] = await useOptions(oysternetOptions).setup('password');
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
*/

interface Options {
  readonly httpUrl: string
  readonly networkId: string
  readonly feeToken: string
  readonly gasPrice: GasPrice
  readonly bech32prefix: string
  readonly hdPath: readonly Slip10RawIndex[]
  readonly faucetUrl?: string
  readonly defaultKeyFile: string
  readonly gasLimits: Partial<GasLimits<CosmWasmFeeTable>> // only set the ones you want to override
}

const oysternetOptions: Options = {
  httpUrl: 'http://rpc.oysternet.cosmwasm.com',
  networkId: 'oysternet-1',
  gasPrice:  GasPrice.fromString("0.01usponge"),
  bech32prefix: 'wasm',
  feeToken: 'usponge',
  faucetUrl: 'https://faucet.oysternet.cosmwasm.com/credit',
  hdPath: makeCosmoshubPath(0),
  defaultKeyFile: path.join(process.env.HOME, ".oysternet.key"),
  gasLimits: {
    upload: 1500000,
    init: 600000,
    register:800000,
    transfer: 80000,
  },
}

interface Network {
  setup: (password: string, filename?: string) => Promise<[string, SigningCosmWasmClient]>
  recoverMnemonic: (password: string, filename?: string) => Promise<string>
}

const useOptions = (options: Options): Network => {

  const loadOrCreateWallet = async (options: Options, filename: string, password: string): Promise<Secp256k1HdWallet> => {
    let encrypted: string;
    try {
      encrypted = fs.readFileSync(filename, 'utf8');
    } catch (err) {
      // generate if no file exists
      const wallet = await Secp256k1HdWallet.generate(12, {hdPaths: [options.hdPath], prefix: options.bech32prefix});
      const encrypted = await wallet.serialize(password);
      fs.writeFileSync(filename, encrypted, 'utf8');
      return wallet;
    }
    // otherwise, decrypt the file (we cannot put deserialize inside try or it will over-write on a bad password)
    const wallet = await Secp256k1HdWallet.deserialize(encrypted, password);
    return wallet;
  };

  const connect = async (
    wallet: Secp256k1HdWallet,
    options: Options
  ): Promise<SigningCosmWasmClient> => {
    const clientOptions = {
      prefix: options.bech32prefix,
      gasPrice: options.gasPrice,
      gasLimits: options.gasLimits
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
    const wallet = await loadOrCreateWallet(oysternetOptions, keyfile, password);
    const client = await connect(wallet, oysternetOptions);

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
    const wallet = await loadOrCreateWallet(oysternetOptions, keyfile, password);
    return wallet.mnemonic;
  }

  return {setup, recoverMnemonic};
}

class Airdrop {
  private tree: MerkleTree;

  constructor(accounts: Array<{ address: string; amount: string }>) {
    const leaves = accounts.map((a) => keccak256(a.address + a.amount));
    this.tree = new MerkleTree(leaves, keccak256, { sort: true });
  }

  public getMerkleRoot(): string {
    return this.tree.getHexRoot().replace('0x', '');
  }

  public getMerkleProof(account: {
    address: string;
    amount: string;
  }): string[] {
    return this.tree
      .getHexProof(keccak256(account.address + account.amount))
      .map((v) => v.replace('0x', ''));
  }

  public verify(
    proof: string[],
    account: { address: string; amount: string }
  ): boolean {
    let hashBuf = keccak256(account.address + account.amount);

    proof.forEach((proofElem) => {
      const proofBuf = Buffer.from(proofElem, 'hex');

      if (hashBuf < proofBuf) {
        hashBuf = keccak256(Buffer.concat([hashBuf, proofBuf]));
      } else {
        hashBuf = keccak256(Buffer.concat([proofBuf, hashBuf]));
      }
    });

    return this.getMerkleRoot() === hashBuf.toString('hex');
  }
}


/*
  Responses
 */
interface ConfigResponse {
  readonly owner: string;
  readonly cw20_token_address: string;
}

interface MerkleRootResponse {
  readonly stage: number;
  readonly merkle_root: string;
}

interface LatestStageResponse {
  readonly latest_stage: number;
}

interface IsClaimedResponse {
  readonly is_claimed: number;
}

interface CW20AirdropInstance {
  readonly contractAddress: string

  // queries
  config: () => Promise<ConfigResponse>
  merkleRoot: (stage: number) => Promise<MerkleRootResponse>
  latestStage: () => Promise<LatestStageResponse>
  isClaimed: (stage: number, address: string) => Promise<IsClaimedResponse>

  // actions
  updateConfig: (txSigner: string, owner?: string) => Promise<string>
  registerMerkleRoot: (txSigner: string, merkleRoot: string) => Promise<string>
  claim: (txSigner: string, stage: number, amount: string, proof: string[]) => Promise<string>
}

interface CW20AirdropContract {
  // upload a code blob and returns a codeId
  upload: (txSigner: string) => Promise<number>

  // instantiates a cw20 contract
  // codeId must come from a previous deploy
  // label is the public name of the contract in listing
  // if you set admin, you can run migrations on this contract (likely client.txSigner)
  instantiate: (txSigner: string, codeId: number, initMsg: Record<string, unknown>, label: string, admin?: string) => Promise<CW20AirdropInstance>

  use: (contractAddress: string) => CW20AirdropInstance
}

export const CW20 = (client: SigningCosmWasmClient): CW20AirdropContract => {
  const use = (contractAddress: string): CW20AirdropInstance => {
    const config = async (): Promise<ConfigResponse> => {
      return await client.queryContractSmart(contractAddress, {config: { }});
    };

    const merkleRoot = async (stage: number): Promise<MerkleRootResponse> => {
      return client.queryContractSmart(contractAddress, {merkle_root: { stage }});
    };

    const latestStage = async (): Promise<LatestStageResponse> => {
      return client.queryContractSmart(contractAddress, {latest_stage: { }});
    };

    const isClaimed = async (stage: number, address: string): Promise<IsClaimedResponse> => {
      return client.queryContractSmart(contractAddress, {is_claied: { stage, address}});
    };

    const updateConfig = async (txSigner: string, owner?: string): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {update_config: { owner }});
      return result.transactionHash;
    }

    // burns tokens, returns transactionHash
    const registerMerkleRoot = async (txSigner: string, merkleRoot: string): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {register_merkle_root: {merkleRoot}});
      return result.transactionHash;
    }

    const claim = async (txSigner: string, stage: number, amount: string, proof: string[]): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {claim: {stage, amount, proof}});
      return result.transactionHash;
    }

    return {
      contractAddress,
      config,
      merkleRoot,
      latestStage,
      isClaimed,
      updateConfig,
      registerMerkleRoot,
      claim
    };
  }

  const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: 'arraybuffer' })
    if (r.status !== 200) {
      throw new Error(`Download error: ${r.status}`)
    }
    return r.data
  }

  const upload = async (txSigner: string): Promise<number> => {
    const meta = {
      source: "https://github.com/CosmWasm/cosmwasm-plus/tree/v0.7.0/contracts/cw20-merkle-airdrop",
      builder: "cosmwasm/workspace-optimizer:0.11.2"
    };
    const sourceUrl = "https://github.com/CosmWasm/cosmwasm-plus/releases/download/v0.7.0/cw20_merkle_airdrop.wasm";
    const wasm = await downloadWasm(sourceUrl);
    const result = await client.upload(txSigner, wasm, meta);
    return result.codeId;
  }

  const instantiate = async (txSigner: string, codeId: number, initMsg: Record<string, unknown>, label: string, admin?: string): Promise<CW20AirdropInstance> => {
    const result = await client.instantiate(txSigner, codeId, initMsg, label, { memo: `Init ${label}`, admin});
    return use(result.contractAddress);
  }

  return { upload, instantiate, use };
}

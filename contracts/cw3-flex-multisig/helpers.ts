import axios from  "axios";
import fs from "fs";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { GasPrice, calculateFee, StdFee } from "@cosmjs/stargate";
import {  DirectSecp256k1HdWallet, makeCosmoshubPath} from "@cosmjs/proto-signing";
import { Slip10RawIndex } from "@cosmjs/crypto";
import { Coin } from "@cosmjs/amino";
import path from "path";
/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * With these you can easily use the cw721 contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli@^0.26 --init https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/cw3-flex-multisig/helpers.ts
 *
 * Create a client:
 *   const [addr, client] = await useOptions(pebblenetOptions).setup('password');
 *
 * Get the mnemonic:
 *   await useOptions(pebblenetOptions).recoverMnemonic(password);
 *
 * Create contract:
 *   const contract = CW3Flex(client, pebblenetOptions.fees);
 *
 * Upload contract:
 *   const codeId = await contract.upload(addr);
 *
 * Instantiate contract example:
 *   const initMsg = {
 *     name: "Potato Coin",
 *     symbol: "TATER",
 *     minter: addr
 *   };
 *   const instance = await contract.instantiate(addr, codeId, initMsg, 'Potato Coin!');
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
    await axios.post(faucetUrl, { denom, address });
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

  return { setup, recoverMnemonic };
}

type ThresholdResponse = AbsoluteCount | AbsolutePercentage | ThresholdQuorum;

// ThresholdResponse Variant
interface AbsoluteCount {
  readonly total_weight: number;
  readonly weight: number;
}

// ThresholdResponse Variant
interface AbsolutePercentage {
  readonly total_weight: number;
  // decimal
  readonly percentage: string;
}

// ThresholdResponse Variant
interface ThresholdQuorum {
  // decimal
  readonly quorum: string;
  // decimal
  readonly threshold: string;
  readonly total_weight: string;
}

interface ProposalResponse {
  readonly id: number;
  readonly title: string;
  readonly description: string;
}

interface ProposalListResponse {
  readonly proposals: ProposalResponse[];
}

enum Vote {
  yes = "yes",
  no = "no",
  abstain = "abstain",
  veto = "veto",
}

interface VoteInfo {
  readonly voter: string;
  readonly vote: Vote;
  readonly weight: number;
}

interface VoteResponse {
  readonly vote?: VoteInfo;
}

interface VoteListResponse {
  readonly votes: VoteInfo[];
}

interface VoterResponse {
  readonly weight?: number;
}

interface VoterListResponse {
  readonly voters: VoterDetail[];
}

interface VoterDetail {
  readonly addr: string;
  readonly weight: number;
}

interface HooksResponse {
  readonly hooks: string[];
}

interface MemberDiff {
  readonly key: string;
  readonly old?: number;
  readonly new?: number;
}

interface MemberChangedHookMsg {
  readonly diffs: MemberDiff[];
}

type CosmosMsg = SendMsg | DelegateMsg | UndelegateMsg | RedelegateMsg | WithdrawMsg | any

interface SendMsg {
  readonly bank: {
    readonly send: {
      readonly from_address: string,
      readonly to_address: string,
      readonly amount: readonly Coin[],
    }
  }
}

interface DelegateMsg {
  readonly staking: {
    readonly delegate: {
      readonly validator: string,
      readonly amount: Coin,
    }
  }
}

interface UndelegateMsg {
  readonly staking: {
    readonly undelegate: {
      readonly validator: string,
      readonly amount: Coin,
    }
  }
}

interface RedelegateMsg {
  readonly staking: {
    readonly redelegate: {
      readonly src_validator: string,
      readonly dst_validator: string,
      readonly amount: Coin,
    }
  }
}

interface WithdrawMsg {
  readonly staking: {
    readonly withdraw: {
      readonly validator: string,
      readonly recipient?: string,
    }
  }
}

type Expiration = { readonly at_height: number } | { readonly at_time: number } | { readonly never: {} };

interface CW3FlexInstance {
  readonly contractAddress: string

  // queries
  threshold: () => Promise<ThresholdResponse>
  proposal: (proposalId: number) => Promise<ProposalResponse>
  query_vote: (proposalId: number, voter: string) => Promise<VoteResponse>
  listProposals: (startAfter?: string, limit?: number) => Promise<ProposalListResponse>
  reverseProposals: (startAfter?: string, limit?: number) => Promise<ProposalListResponse>
  listVotes: (proposalId: number, startAfter?: string, limit?: number) => Promise<VoteListResponse>
  voter: (address: string) => Promise<VoterDetail>
  listVoters: (startAfter?: string, limit?: number) => Promise<VoterListResponse>

  // actions
  propose: (txSigner: string, description: string, msgs: CosmosMsg[], latest?: Expiration) => Promise<string>
  vote: (txSigner: string, proposalId: number, vote: Vote) => Promise<string>
  execute: (txSigner: string, proposalId: number) => Promise<string>
  close: (txSigner: string, proposalId: number) => Promise<string>
  // should be triggered by other contract, use for testing
  _memberChangedHook: (txSigner: string, diffs: MemberDiff[]) => Promise<string>
}

interface CW3FlexContract {
  upload: (txSigner: string) => Promise<number>
  instantiate: (txSigner: string, codeId: number, initMsg: Record<string, unknown>, label: string, admin?: string) => Promise<CW3FlexInstance>
  use: (contractAddress: string) => CW3FlexInstance
}

export const CW3Flex = (client: SigningCosmWasmClient, fees: Options['fees']): CW3FlexContract => {
  const use = (contractAddress: string): CW3FlexInstance => {

    const threshold = async (): Promise<ThresholdResponse> => {
      return client.queryContractSmart(contractAddress, {threshold: {}});
    };

    const proposal = async (): Promise<ProposalResponse> => {
      return client.queryContractSmart(contractAddress, {proposal: {}});
    };

    const query_vote = async (proposalId: number, voter: string): Promise<VoteResponse> => {
      return client.queryContractSmart(contractAddress, {vote: {proposal_id: proposalId, voter}});
    };

    const listProposals = async (startAfter?: string, limit?: number): Promise<ProposalListResponse> => {
      return client.queryContractSmart(contractAddress, {list_proposals: {start_after: startAfter, limit}});
    };

    const reverseProposals = async (startAfter?: string, limit?: number): Promise<ProposalListResponse> => {
      return client.queryContractSmart(contractAddress, {reverse_proposals: {start_after: startAfter, limit}});
    };

    const listVotes = async (proposalId: number, startAfter?: string, limit?: number): Promise<VoteListResponse> => {
      return client.queryContractSmart(contractAddress, {
        list_votes: {
          proposal_id: proposalId,
          start_after: startAfter,
          limit
        }
      });
    }

    const voter = async (address: string): Promise<VoterDetail> => {
      return client.queryContractSmart(contractAddress, {voter: {address}});
    }

    const listVoters = async (startAfter?: string, limit?: number): Promise<VoterListResponse> => {
      return client.queryContractSmart(contractAddress, {list_voters: {start_after: startAfter, limit}});
    }

    const propose = async (txSigner: string, description: string, msgs: CosmosMsg[], latest?: Expiration): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {propose: {description, msgs, latest}}, fees.exec);
      return result.transactionHash;
    }

    const vote = async (txSigner: string, proposalId: number, vote: Vote): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {vote: {proposal_id: proposalId, vote}}, fees.exec);
      return result.transactionHash;
    }

    const execute = async (txSigner: string, proposalId: number): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {execute: {proposal_id: proposalId}}, fees.exec);
      return result.transactionHash;
    }

    const close = async (txSigner: string, proposalId: number): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {close: {proposal_id: proposalId}}, fees.exec);
      return result.transactionHash;
    }

    const _memberChangedHook = async (txSigner: string, diffs: MemberDiff[]): Promise<string> => {
      const result = await client.execute(txSigner, contractAddress, {membership_hook: {diffs: diffs}}, fees.exec);
      return result.transactionHash;
    }

    return {
      contractAddress,
      threshold,
      proposal,
      query_vote,
      listProposals,
      reverseProposals,
      voter,
      listVoters,
      listVotes,
      propose,
      vote,
      execute,
      close,
      _memberChangedHook
    };
  }

  const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, {responseType: 'arraybuffer'})
    if (r.status !== 200) {
      throw new Error(`Download error: ${r.status}`)
    }
    return r.data
  }

  const upload = async (senderAddress: string): Promise<number> => {
    const sourceUrl = "https://github.com/CosmWasm/cw-plus/releases/download/v0.9.0/cw3_flex_multisig.wasm";
    const wasm = await downloadWasm(sourceUrl);
    const result = await client.upload(senderAddress, wasm, fees.upload);
    return result.codeId;
  }

  const instantiate = async (senderAddress: string, codeId: number, initMsg: Record<string, unknown>, label: string, admin?: string): Promise<CW3FlexInstance> => {
    const result = await client.instantiate(senderAddress, codeId, initMsg, label, fees.init, { memo: `Init ${label}`, admin });
    return use(result.contractAddress);
  }

  return {upload, instantiate, use};
}

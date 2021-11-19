import { Coin } from "@cosmjs/amino"
import { calculateFee } from "@cosmjs/stargate"
/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * Look at https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/base-helpers.ts on how to setup a wallet
 * With these you can easily use the cw3 contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli@^0.26 --init https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/base-helpers.ts --init https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/cw3-flex-multisig/helpers.ts
 *
 * Create a client:
 *   const [addr, client] = await useOptions(uniOptions).setup('password');
 *
 * Get the mnemonic:
 *   await useOptions(uniOptions).recoverMnemonic(password);
 *
 * Create contract:
 *   const contract = CW3Flex(client, uniOptions);
 *
 * Upload contract:
 *   const codeId = await contract.upload(addr, uniOptions);
 *
 * In order to instantiate a cw3-flex-multisig contract;
 * You need to instantiate a cw4-group contract and use it's contract address as the group_addr in initMsg
 * 
 * Instantiate contract example:
 *   const initMsg = {
 *     group_addr: "juno1guv2ra0ryj4jvnxf2efeu8g0a2mewr979gfa67x033an638nj33qy09jvu", // cw4-group contract address
 *     threshold: {
 *        absolute_count: {
 *          weight: 5
 *        }
 *     },
 *     max_voting_period: {
 *        time: 3600
 *     }
 *   };
 * 
 *   const instance = await contract.instantiate(addr, codeId, initMsg, 'Potato Coin!', uniOptions);
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */

type ThresholdResponse = AbsoluteCount | AbsolutePercentage | ThresholdQuorum

// ThresholdResponse Variant
interface AbsoluteCount {
  readonly total_weight: number
  readonly weight: number
}

// ThresholdResponse Variant
interface AbsolutePercentage {
  readonly total_weight: number
  // decimal
  readonly percentage: string
}

// ThresholdResponse Variant
interface ThresholdQuorum {
  // decimal
  readonly quorum: string
  // decimal
  readonly threshold: string
  readonly total_weight: string
}

interface ProposalResponse {
  readonly id: number
  readonly title: string
  readonly description: string
}

interface ProposalListResponse {
  readonly proposals: ProposalResponse[]
}

interface VoteInfo {
  readonly voter: string
  readonly vote: string
  readonly weight: number
}

interface VoteResponse {
  readonly vote?: VoteInfo
}

interface VoteListResponse {
  readonly votes: VoteInfo[]
}

interface VoterListResponse {
  readonly voters: VoterDetail[]
}

interface VoterDetail {
  readonly addr: string
  readonly weight: number
}

interface MemberDiff {
  readonly key: string
  readonly old?: number
  readonly new?: number
}

type CosmosMsg = SendMsg | DelegateMsg | UndelegateMsg | RedelegateMsg | WithdrawMsg | any

interface SendMsg {
  readonly bank: {
    readonly send: {
      readonly from_address: string
      readonly to_address: string
      readonly amount: readonly Coin[]
    }
  }
}

interface DelegateMsg {
  readonly staking: {
    readonly delegate: {
      readonly validator: string
      readonly amount: Coin
    }
  }
}

interface UndelegateMsg {
  readonly staking: {
    readonly undelegate: {
      readonly validator: string
      readonly amount: Coin
    }
  }
}

interface RedelegateMsg {
  readonly staking: {
    readonly redelegate: {
      readonly src_validator: string
      readonly dst_validator: string
      readonly amount: Coin
    }
  }
}

interface WithdrawMsg {
  readonly staking: {
    readonly withdraw: {
      readonly validator: string
      readonly recipient?: string
    }
  }
}

type Expiration = { readonly at_height: number } | { readonly at_time: number } | { readonly never: {} }

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
  vote: (txSigner: string, proposalId: number, vote: string) => Promise<string>
  execute: (txSigner: string, proposalId: number) => Promise<string>
  close: (txSigner: string, proposalId: number) => Promise<string>
  // should be triggered by other contract, use for testing
  _memberChangedHook: (txSigner: string, diffs: MemberDiff[]) => Promise<string>
}

interface CW3FlexContract {
  upload: (txSigner: string, options: Options) => Promise<number>
  instantiate: (
    txSigner: string,
    codeId: number,
    initMsg: Record<string, unknown>,
    label: string,
    options: Options,
    admin?: string,
  ) => Promise<CW3FlexInstance>
  use: (contractAddress: string) => CW3FlexInstance
}

export const CW3Flex = (client: SigningCosmWasmClient, options: Options): CW3FlexContract => {
  const use = (contractAddress: string): CW3FlexInstance => {
    const threshold = async (): Promise<ThresholdResponse> => {
      return client.queryContractSmart(contractAddress, { threshold: {} })
    }

    const proposal = async (): Promise<ProposalResponse> => {
      return client.queryContractSmart(contractAddress, { proposal: {} })
    }

    const query_vote = async (proposalId: number, voter: string): Promise<VoteResponse> => {
      return client.queryContractSmart(contractAddress, {
        vote: { proposal_id: proposalId, voter },
      })
    }

    const listProposals = async (startAfter?: string, limit?: number): Promise<ProposalListResponse> => {
      return client.queryContractSmart(contractAddress, {
        list_proposals: { start_after: startAfter, limit },
      })
    }

    const reverseProposals = async (startAfter?: string, limit?: number): Promise<ProposalListResponse> => {
      return client.queryContractSmart(contractAddress, {
        reverse_proposals: { start_after: startAfter, limit },
      })
    }

    const listVotes = async (proposalId: number, startAfter?: string, limit?: number): Promise<VoteListResponse> => {
      return client.queryContractSmart(contractAddress, {
        list_votes: {
          proposal_id: proposalId,
          start_after: startAfter,
          limit,
        },
      })
    }

    const voter = async (address: string): Promise<VoterDetail> => {
      return client.queryContractSmart(contractAddress, { voter: { address } })
    }

    const listVoters = async (startAfter?: string, limit?: number): Promise<VoterListResponse> => {
      return client.queryContractSmart(contractAddress, {
        list_voters: { start_after: startAfter, limit },
      })
    }

    const propose = async (
      txSigner: string,
      description: string,
      msgs: CosmosMsg[],
      latest?: Expiration,
    ): Promise<string> => {
      const fee = calculateFee(options.fees.exec, options.gasPrice)

      const result = await client.execute(txSigner, contractAddress, { propose: { description, msgs, latest } }, fee)
      return result.transactionHash
    }

    const vote = async (txSigner: string, proposalId: number, vote: string): Promise<string> => {
      const fee = calculateFee(options.fees.exec, options.gasPrice)

      const result = await client.execute(txSigner, contractAddress, { vote: { proposal_id: proposalId, vote } }, fee)
      return result.transactionHash
    }

    const execute = async (txSigner: string, proposalId: number): Promise<string> => {
      const fee = calculateFee(options.fees.exec, options.gasPrice)

      const result = await client.execute(txSigner, contractAddress, { execute: { proposal_id: proposalId } }, fee)
      return result.transactionHash
    }

    const close = async (txSigner: string, proposalId: number): Promise<string> => {
      const fee = calculateFee(options.fees.exec, options.gasPrice)

      const result = await client.execute(txSigner, contractAddress, { close: { proposal_id: proposalId } }, fee)
      return result.transactionHash
    }

    const _memberChangedHook = async (txSigner: string, diffs: MemberDiff[]): Promise<string> => {
      const fee = calculateFee(options.fees.exec, options.gasPrice)

      const result = await client.execute(txSigner, contractAddress, { membership_hook: { diffs: diffs } }, fee)
      return result.transactionHash
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
      _memberChangedHook,
    }
  }

  const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: "arraybuffer" })
    if (r.status !== 200) {
      throw new Error(`Download error: ${r.status}`)
    }
    return r.data
  }

  const upload = async (senderAddress: string, options: Options): Promise<number> => {
    const sourceUrl = "https://github.com/CosmWasm/cw-plus/releases/download/v0.10.2/cw3_flex_multisig.wasm"
    const wasm = await downloadWasm(sourceUrl)
    const fee = calculateFee(options.fees.upload * 2, options.gasPrice)
    const result = await client.upload(senderAddress, wasm, fee)
    return result.codeId
  }

  const instantiate = async (
    senderAddress: string,
    codeId: number,
    initMsg: Record<string, unknown>,
    label: string,
    options: Options,
    admin?: string,
  ): Promise<CW3FlexInstance> => {
    const fee = calculateFee(options.fees.init, options.gasPrice)
    const result = await client.instantiate(senderAddress, codeId, initMsg, label, fee, {
      memo: `Init ${label}`,
      admin,
    })
    return use(result.contractAddress)
  }

  return { upload, instantiate, use }
}

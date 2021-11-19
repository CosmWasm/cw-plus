import { calculateFee } from "@cosmjs/stargate"

/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * Look at https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/base-helpers.ts on how to setup a wallet
 * With these you can easily use the cw20 contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli@^0.26 --init https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/base-helpers.ts --init https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/cw4-group/helpers.ts
 *
 * Create a client:
 *   const [addr, client] = await useOptions(uniOptions).setup('password');
 *
 * Get the mnemonic:
 *   await useOptions(uniOptions).recoverMnemonic(password);
 *
 * Create contract:
 *   const contract = CW4Group(client, uniOptions);
 *
 * Upload contract:
 *   const codeId = await contract.upload(addr, uniOptions);
 *
 * Instantiate contract example:
 *   const initMsg = {
 *     admin: addr,
 *     members: [
 *       {
 *          addr: "juno17r7l8v2nlxpf53p4zgdlq9trw0ndw5x3jp98yh",
 *          weight: 10,
 *       },
 *       {
 *          addr: "juno10h52xd7gec64zp6ct6qfu3lhhnuv4lgz7srs8g",
 *          weight: 15,
 *       },
 *     ]
 *   };
 *   const instance = await contract.instantiate(addr, codeId, initMsg, 'Potato Coin!', uniOptions);
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */

interface AdminResponse {
  readonly admin?: string
}

interface MemberResponse {
  readonly weight?: number
}

interface MemberListResponse {
  readonly members: Member[]
}

interface Member {
  readonly addr: string
  readonly weight: number
}

interface TotalWeightResponse {
  readonly weight: number
}

interface HooksResponse {
  readonly hooks: readonly string[]
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
  updateMembers: (txSigner: string, remove: Member[], add: Member[]) => Promise<string>

  // will not used by end user for testing purposes
  _addHook: (txSigner: string, addr: string) => Promise<string>
  _removeHook: (txSigner: string, addr: string) => Promise<string>
}

interface CW4GroupContract {
  upload: (txSigner: string, options: Options) => Promise<number>
  instantiate: (
    txSigner: string,
    codeId: number,
    initMsg: Record<string, unknown>,
    label: string,
    options: Options,
    admin?: string,
  ) => Promise<CW4GroupInstance>
  use: (contractAddress: string) => CW4GroupInstance
}

export const CW4Group = (client: SigningCosmWasmClient, options: Options): CW4GroupContract => {
  const use = (contractAddress: string): CW4GroupInstance => {
    const admin = async (): Promise<AdminResponse> => {
      return client.queryContractSmart(contractAddress, { admin: {} })
    }

    const totalWeight = async (): Promise<TotalWeightResponse> => {
      return client.queryContractSmart(contractAddress, { total_weight: {} })
    }

    const member = async (addr: string, atHeight?: number): Promise<MemberResponse> => {
      return client.queryContractSmart(contractAddress, { member: { addr, at_height: atHeight } })
    }

    const listMembers = async (startAfter?: string, limit?: number): Promise<MemberListResponse> => {
      return client.queryContractSmart(contractAddress, { list_members: { start_after: startAfter, limit } })
    }

    const hooks = async (): Promise<HooksResponse> => {
      return client.queryContractSmart(contractAddress, { hooks: {} })
    }

    const updateAdmin = async (txSigner: string, admin?: string): Promise<string> => {
      const fee = calculateFee(options.fees.exec, options.gasPrice)

      const result = await client.execute(txSigner, contractAddress, { update_admin: { admin } }, fee)
      return result.transactionHash
    }

    const updateMembers = async (txSigner: string, remove: Member[], add: Member[]): Promise<string> => {
      const fee = calculateFee(options.fees.exec, options.gasPrice)

      const result = await client.execute(txSigner, contractAddress, { update_members: { remove, add } }, fee)
      return result.transactionHash
    }

    const _addHook = async (txSigner: string, addr: string): Promise<string> => {
      const fee = calculateFee(options.fees.exec, options.gasPrice)

      const result = await client.execute(txSigner, contractAddress, { add_hook: { addr } }, fee)
      return result.transactionHash
    }

    const _removeHook = async (txSigner: string, addr: string): Promise<string> => {
      const fee = calculateFee(options.fees.exec, options.gasPrice)

      const result = await client.execute(txSigner, contractAddress, { remove_hook: { addr } }, fee)
      return result.transactionHash
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
      _addHook,
      _removeHook,
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
    const sourceUrl = "https://github.com/CosmWasm/cw-plus/releases/download/v0.10.2/cw4_group.wasm"
    const wasm = await downloadWasm(sourceUrl)
    const fee = calculateFee(options.fees.upload, options.gasPrice)
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
  ): Promise<CW4GroupInstance> => {
    const fee = calculateFee(options.fees.init, options.gasPrice)
    const result = await client.instantiate(senderAddress, codeId, initMsg, label, fee, {
      memo: `Init ${label}`,
      admin,
    })
    return use(result.contractAddress)
  }

  return { upload, instantiate, use }
}

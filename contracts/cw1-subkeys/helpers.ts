import { Coin } from "@cosmjs/amino";

/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * Look at https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/base-helpers.ts on how to setup a wallet
 * With these you can easily use the cw1 contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli@^0.26 --init https://github.com/CosmWasm/cw-plus/blob/master/contracts/base-helpers.ts --init https://github.com/CosmWasm/cw-plus/blob/master/contracts/cw1-subkeys/helpers.ts
 *
 * Create a client:
 *   const [addr, client] = await useOptions(pebblenetOptions).setup('password');
 *
 * Get the mnemonic:
 *   await useOptions(pebblenetOptions).recoverMnemonic(password);
 *
 * Create contract:
 *   const contract = CW1(client, pebblenetOptions.fees);
 *
 * Upload contract:
 *   const codeId = await contract.upload(addr);
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */

type Expiration = { at_height: { height: number } } | { at_time: { time: number } } | { never: {} };

interface CanExecuteResponse {
  readonly canExecute: boolean;
}

interface Permissions {
  readonly delegate: boolean;
  readonly undelegate: boolean;
  readonly redelegate: boolean;
  readonly withdraw: boolean;
}

interface PermissionsInfo {
  readonly spender: string;
  readonly permissions: Permissions;
}

interface AllPermissionsResponse {
  readonly permissions: readonly PermissionsInfo[];
}

interface AllowanceInfo {
  readonly balance: readonly Coin[];
  readonly expires: Expiration;
}

interface AllAllowancesResponse {
  readonly allowances: readonly AllowanceInfo[];
}

interface AdminListResponse {
  readonly admins: readonly string[];
  readonly mutable: boolean;
}

type CosmosMsg = SendMsg | DelegateMsg | UndelegateMsg | RedelegateMsg | WithdrawMsg;

interface SendMsg {
  readonly bank: {
    readonly send: {
      readonly from_address: string;
      readonly to_address: string;
      readonly amount: readonly Coin[];
    };
  };
}

interface DelegateMsg {
  readonly staking: {
    readonly delegate: {
      readonly validator: string;
      readonly amount: Coin;
    };
  };
}

interface UndelegateMsg {
  readonly staking: {
    readonly undelegate: {
      readonly validator: string;
      readonly amount: Coin;
    };
  };
}

interface RedelegateMsg {
  readonly staking: {
    readonly redelegate: {
      readonly src_validator: string;
      readonly dst_validator: string;
      readonly amount: Coin;
    };
  };
}

interface WithdrawMsg {
  readonly staking: {
    readonly withdraw: {
      readonly validator: string;
      readonly recipient?: string;
    };
  };
}

interface CW1Instance {
  readonly contractAddress: string;

  // queries
  admins: () => Promise<AdminListResponse>;
  allowance: (address?: string) => Promise<AllowanceInfo>;
  allAllowances: (startAfter?: string, limit?: number) => Promise<AllAllowancesResponse>;

  permissions: (address?: string) => Promise<PermissionsInfo>;
  allPermissions: (startAfter?: string, limit?: number) => Promise<AllPermissionsResponse>;
  canExecute: (sender: string, msg: CosmosMsg) => Promise<CanExecuteResponse>;

  // actions
  execute: (senderAddress: string, msgs: readonly CosmosMsg[]) => Promise<string>;
  freeze: (senderAddress: string) => Promise<string>;
  updateAdmins: (senderAddress: string, admins: readonly string[]) => Promise<string>;
  increaseAllowance: (senderAddress: string, recipient: string, amount: Coin, expires?: Expiration) => Promise<string>;
  decreaseAllowance: (senderAddress: string, recipient: string, amount: Coin, expires?: Expiration) => Promise<string>;
  setPermissions: (senderAddress: string, recipient: string, permissions: Permissions) => Promise<string>;
}

interface CW1Contract {
  upload: (senderAddress: string) => Promise<number>;

  instantiate: (
    senderAddress: string,
    codeId: number,
    initMsg: Record<string, unknown>,
    label: string,
    admin?: string
  ) => Promise<CW1Instance>;

  use: (contractAddress: string) => CW1Instance;
}

const CW1 = (client: SigningCosmWasmClient, fees: Options["fees"]): CW1Contract => {
  const use = (contractAddress: string): CW1Instance => {
    const allowance = async (address?: string): Promise<AllowanceInfo> => {
      return await client.queryContractSmart(contractAddress, { allowance: { address } });
    };

    const allAllowances = async (startAfter?: string, limit?: number): Promise<AllAllowancesResponse> => {
      return client.queryContractSmart(contractAddress, { all_allowances: { start_after: startAfter, limit: limit } });
    };

    const permissions = async (address?: string): Promise<PermissionsInfo> => {
      return await client.queryContractSmart(contractAddress, { permissions: { address } });
    };

    const allPermissions = async (startAfter?: string, limit?: number): Promise<AllPermissionsResponse> => {
      return client.queryContractSmart(contractAddress, { all_permissions: { start_after: startAfter, limit: limit } });
    };

    const canExecute = async (sender: string, msg: CosmosMsg): Promise<CanExecuteResponse> => {
      return client.queryContractSmart(contractAddress, { can_execute: { sender: sender, msg: msg } });
    };

    const admins = async (): Promise<AdminListResponse> => {
      return client.queryContractSmart(contractAddress, { admin_list: {} });
    };

    // called by an admin to make admin set immutable
    const freeze = async (senderAddress: string): Promise<string> => {
      const result = await client.execute(senderAddress, contractAddress, { freeze: {} }, fees.exec);
      return result.transactionHash;
    };

    // burns tokens, returns transactionHash
    const updateAdmins = async (senderAddress: string, admins: readonly string[]): Promise<string> => {
      const result = await client.execute(senderAddress, contractAddress, { update_admins: { admins } }, fees.exec);
      return result.transactionHash;
    };

    // transfers tokens, returns transactionHash
    const execute = async (senderAddress: string, msgs: readonly CosmosMsg[]): Promise<string> => {
      const result = await client.execute(senderAddress, contractAddress, { execute: { msgs } }, fees.exec);
      return result.transactionHash;
    };

    const increaseAllowance = async (
      senderAddress: string,
      spender: string,
      amount: Coin,
      expires?: Expiration
    ): Promise<string> => {
      const result = await client.execute(
        senderAddress,
        contractAddress,
        {
          increase_allowance: { spender, amount, expires },
        },
        fees.exec
      );
      return result.transactionHash;
    };

    const decreaseAllowance = async (
      senderAddress: string,
      spender: string,
      amount: Coin,
      expires?: Expiration
    ): Promise<string> => {
      const result = await client.execute(
        senderAddress,
        contractAddress,
        {
          decrease_allowance: { spender, amount, expires },
        },
        fees.exec
      );
      return result.transactionHash;
    };

    const setPermissions = async (
      senderAddress: string,
      spender: string,
      permissions: Permissions
    ): Promise<string> => {
      const result = await client.execute(
        senderAddress,
        contractAddress,
        {
          set_permissions: { spender, permissions },
        },
        fees.exec
      );
      return result.transactionHash;
    };

    return {
      contractAddress,
      admins,
      allowance,
      allAllowances,
      permissions,
      allPermissions,
      canExecute,
      execute,
      freeze,
      updateAdmins,
      increaseAllowance,
      decreaseAllowance,
      setPermissions,
    };
  };

  const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: "arraybuffer" });
    if (r.status !== 200) {
      throw new Error(`Download error: ${r.status}`);
    }
    return r.data;
  };

  const upload = async (senderAddress: string): Promise<number> => {
    const sourceUrl = "https://github.com/CosmWasm/cw-plus/releases/download/v0.9.1/cw1_subkeys.wasm";
    const wasm = await downloadWasm(sourceUrl);
    const result = await client.upload(senderAddress, wasm, fees.upload);
    return result.codeId;
  };

  const instantiate = async (
    senderAddress: string,
    codeId: number,
    initMsg: Record<string, unknown>,
    label: string,
    admin?: string
  ): Promise<CW1Instance> => {
    const result = await client.instantiate(senderAddress, codeId, initMsg, label, fees.init, {
      memo: `Init ${label}`,
      admin,
    });
    return use(result.contractAddress);
  };

  return { upload, instantiate, use };
};

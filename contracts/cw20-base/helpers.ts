import { toUtf8, toBase64 } from "@cosmjs/encoding";

/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * Look at https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/base-helpers.ts on how to setup a wallet
 * With these you can easily use the cw20 contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli@^0.26 --init https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/base-helpers.ts --init https://raw.githubusercontent.com/CosmWasm/cw-plus/master/contracts/cw20-base/helpers.ts
 *
 * Create a client:
 *   const [addr, client] = await useOptions(pebblenetOptions).setup('password');
 *
 * Get the mnemonic:
 *   await useOptions(pebblenetOptions).recoverMnemonic(password);
 *
 * Create contract:
 *   const contract = CW20(client, pebblenetOptions.fees);
 *
 * Upload contract:
 *   const codeId = await contract.upload(addr);
 *
 * Instantiate contract example:
 *   const initMsg = {
 *     name: "Potato Coin",
 *     symbol: "TATER",
 *     decimals: 2,
 *     initial_balances: [{ address: addr, amount: "10000" }],
 *     mint: { "minter": addr }
 *   };
 *   const instance = await contract.instantiate(addr, codeId, initMsg, 'Potato Coin!');
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */

type Expiration = { readonly at_height: number } | { readonly at_time: number } | { readonly never: {} };

interface AllowanceResponse {
  readonly allowance: string; // integer as string
  readonly expires: Expiration;
}

interface AllowanceInfo {
  readonly allowance: string; // integer as string
  readonly spender: string; // bech32 address
  readonly expires: Expiration;
}

interface AllAllowancesResponse {
  readonly allowances: readonly AllowanceInfo[];
}

interface AllAccountsResponse {
  // list of bech32 address that have a balance
  readonly accounts: readonly string[];
}

interface CW20Instance {
  readonly contractAddress: string;

  // queries
  balance: (address?: string) => Promise<string>;
  allowance: (owner: string, spender: string) => Promise<AllowanceResponse>;
  allAllowances: (owner: string, startAfter?: string, limit?: number) => Promise<AllAllowancesResponse>;
  allAccounts: (startAfter?: string, limit?: number) => Promise<readonly string[]>;
  tokenInfo: () => Promise<any>;
  minter: () => Promise<any>;

  // actions
  mint: (txSigner: string, recipient: string, amount: string) => Promise<string>;
  transfer: (txSigner: string, recipient: string, amount: string) => Promise<string>;
  send: (txSigner: string, recipient: string, amount: string, msg: Record<string, unknown>) => Promise<string>;
  burn: (txSigner: string, amount: string) => Promise<string>;
  increaseAllowance: (txSigner: string, recipient: string, amount: string) => Promise<string>;
  decreaseAllowance: (txSigner: string, recipient: string, amount: string) => Promise<string>;
  transferFrom: (txSigner: string, owner: string, recipient: string, amount: string) => Promise<string>;
  sendFrom: (
    txSigner: string,
    owner: string,
    recipient: string,
    amount: string,
    msg: Record<string, unknown>
  ) => Promise<string>;
}

interface CW20Contract {
  // upload a code blob and returns a codeId
  upload: (txSigner: string) => Promise<number>;

  // instantiates a cw20 contract
  // codeId must come from a previous deploy
  // label is the public name of the contract in listing
  // if you set admin, you can run migrations on this contract (likely client.senderAddress)
  instantiate: (
    txSigner: string,
    codeId: number,
    initMsg: Record<string, unknown>,
    label: string,
    admin?: string
  ) => Promise<CW20Instance>;

  use: (contractAddress: string) => CW20Instance;
}

export const CW20 = (client: SigningCosmWasmClient, fees: Options["fees"]): CW20Contract => {
  const use = (contractAddress: string): CW20Instance => {
    const balance = async (address: string): Promise<string> => {
      const result = await client.queryContractSmart(contractAddress, { balance: { address } });
      return result.balance;
    };

    const allowance = async (owner: string, spender: string): Promise<AllowanceResponse> => {
      return client.queryContractSmart(contractAddress, { allowance: { owner, spender } });
    };

    const allAllowances = async (
      owner: string,
      startAfter?: string,
      limit?: number
    ): Promise<AllAllowancesResponse> => {
      return client.queryContractSmart(contractAddress, { all_allowances: { owner, start_after: startAfter, limit } });
    };

    const allAccounts = async (startAfter?: string, limit?: number): Promise<readonly string[]> => {
      const accounts: AllAccountsResponse = await client.queryContractSmart(contractAddress, {
        all_accounts: { start_after: startAfter, limit },
      });
      return accounts.accounts;
    };

    const tokenInfo = async (): Promise<any> => {
      return client.queryContractSmart(contractAddress, { token_info: {} });
    };

    const minter = async (): Promise<any> => {
      return client.queryContractSmart(contractAddress, { minter: {} });
    };

    // mints tokens, returns transactionHash
    const mint = async (senderAddress: string, recipient: string, amount: string): Promise<string> => {
      const result = await client.execute(senderAddress, contractAddress, { mint: { recipient, amount } }, fees.exec);
      return result.transactionHash;
    };

    // transfers tokens, returns transactionHash
    const transfer = async (senderAddress: string, recipient: string, amount: string): Promise<string> => {
      const result = await client.execute(
        senderAddress,
        contractAddress,
        { transfer: { recipient, amount } },
        fees.exec
      );
      return result.transactionHash;
    };

    // burns tokens, returns transactionHash
    const burn = async (senderAddress: string, amount: string): Promise<string> => {
      const result = await client.execute(senderAddress, contractAddress, { burn: { amount } }, fees.exec);
      return result.transactionHash;
    };

    const increaseAllowance = async (senderAddress: string, spender: string, amount: string): Promise<string> => {
      const result = await client.execute(
        senderAddress,
        contractAddress,
        { increase_allowance: { spender, amount } },
        fees.exec
      );
      return result.transactionHash;
    };

    const decreaseAllowance = async (senderAddress: string, spender: string, amount: string): Promise<string> => {
      const result = await client.execute(
        senderAddress,
        contractAddress,
        { decrease_allowance: { spender, amount } },
        fees.exec
      );
      return result.transactionHash;
    };

    const transferFrom = async (
      senderAddress: string,
      owner: string,
      recipient: string,
      amount: string
    ): Promise<string> => {
      const result = await client.execute(
        senderAddress,
        contractAddress,
        { transfer_from: { owner, recipient, amount } },
        fees.exec
      );
      return result.transactionHash;
    };

    const jsonToBinary = (json: Record<string, unknown>): string => {
      return toBase64(toUtf8(JSON.stringify(json)));
    };

    const send = async (
      senderAddress: string,
      recipient: string,
      amount: string,
      msg: Record<string, unknown>
    ): Promise<string> => {
      const result = await client.execute(
        senderAddress,
        contractAddress,
        { send: { recipient, amount, msg: jsonToBinary(msg) } },
        fees.exec
      );
      return result.transactionHash;
    };

    const sendFrom = async (
      senderAddress: string,
      owner: string,
      recipient: string,
      amount: string,
      msg: Record<string, unknown>
    ): Promise<string> => {
      const result = await client.execute(
        senderAddress,
        contractAddress,
        { send_from: { owner, recipient, amount, msg: jsonToBinary(msg) } },
        fees.exec
      );
      return result.transactionHash;
    };

    return {
      contractAddress,
      balance,
      allowance,
      allAllowances,
      allAccounts,
      tokenInfo,
      minter,
      mint,
      transfer,
      burn,
      increaseAllowance,
      decreaseAllowance,
      transferFrom,
      send,
      sendFrom,
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
    const sourceUrl = "https://github.com/CosmWasm/cosmwasm-plus/releases/download/v0.8.1/cw20_base.wasm";
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
  ): Promise<CW20Instance> => {
    const result = await client.instantiate(senderAddress, codeId, initMsg, label, fees.init, {
      memo: `Init ${label}`,
      admin,
    });
    return use(result.contractAddress);
  };

  return { upload, instantiate, use };
};

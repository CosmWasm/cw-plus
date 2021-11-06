/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * With these you can easily use the cw20 contract without worrying about forming messages and parsing queries.
 * 
 * Usage: npx @cosmjs/cli@^0.23 --init https://github.com/CosmWasm/cosmwasm-plus/blob/master/contracts/cw1-subkeys/helpers.ts
 * 
 * Create a client:
<<<<<<< HEAD
 *   const [addr, client] = await useOptions(pebblenetOptions).setup('password');
 *
 * Get the mnemonic:
 *   await useOptions(pebblenetOptions).recoverMnemonic(password);
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */

const path = require("path");

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

const hackatomOptions: Options = {
  httpUrl: 'https://lcd.heldernet.cosmwasm.com',
  networkId: 'hackatom-wasm',
  gasPrice:  GasPrice.fromString("0.025ucosm"),
  bech32prefix: 'cosmos',
  feeToken: 'ucosm',
  faucetUrl: 'https://faucet.heldernet.cosmwasm.com/credit',
  hdPath: makeCosmoshubPath(0),
  defaultKeyFile: path.join(process.env.HOME, ".heldernet.key"),
  gasLimits: {
    upload: 1500000,
    init: 600000,
    register:800000,
    transfer: 80000,
  },
}

interface Network {
  setup: (password: string, filename?: string) => Promise<SigningCosmWasmClient>
  recoverMnemonic: (password: string, filename?: string) => Promise<string>
}

const useOptions = (options: Options): Network => {

  const loadOrCreateWallet = async (options: Options, filename: string, password: string): Promise<Secp256k1HdWallet> => {
    let encrypted: string;
    try {
      encrypted = fs.readFileSync(filename, 'utf8');
    } catch (err) {
      // generate if no file exists
      const wallet = await Secp256k1HdWallet.generate(12, options.hdPath, options.bech32prefix);
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
    const [{ address }] = await wallet.getAccounts();

    const client = new SigningCosmWasmClient(
      options.httpUrl,
      address,
      wallet,
      hackatomOptions.gasPrice,
      hackatomOptions.gasLimits,
    );
    return client;
  };

  
  const hitFaucet = async (
    faucetUrl: string,
    address: string,
    ticker: string
  ): Promise<void> => {
    await axios.post(faucetUrl, { ticker, address });
  }
  
  const setup = async (password: string, filename?: string): Promise<SigningCosmWasmClient> => {
    const keyfile = filename || options.defaultKeyFile;
    const wallet = await loadOrCreateWallet(hackatomOptions, keyfile, password);
    const client = await connect(wallet, hackatomOptions);

    // ensure we have some tokens
    if (options.faucetUrl) {
      const account = await client.getAccount();
      if (!account) {
        console.log(`Getting ${options.feeToken} from faucet`);
        await hitFaucet(options.faucetUrl, client.senderAddress, options.feeToken);
      }
    }

    return client;
  }

  const recoverMnemonic = async (password: string, filename?: string): Promise<string> => {
    const keyfile = filename || options.defaultKeyFile;
    const wallet = await loadOrCreateWallet(hackatomOptions, keyfile, password);
    return wallet.mnemonic;
  }

  return {setup, recoverMnemonic};
}

type Expiration = { at_height: { height: number } } | { at_time: { time: number } } | { never: {}}
>>>>>>> parent of f4092706 (Update helper version and refactor based on new base helper)

interface CanExecuteResponse {
  readonly canExecute: boolean
}

  readonly delegate: boolean
  readonly undelegate: boolean
  readonly redelegate: boolean
  readonly withdraw: boolean
}

interface PermissionsInfo {
  readonly spender: string
  readonly permissions: Permissions
}

interface AllPermissionsResponse {
  readonly permissions: readonly PermissionsInfo[]
}

interface AllowanceInfo {
  readonly balance: readonly Coin[],
  readonly expires: Expiration,
}

interface AllAllowancesResponse {
  readonly allowances: readonly AllowanceInfo[]
}

interface AdminListResponse {
  readonly admins: readonly string[],
  readonly mutable: boolean,
}

type CosmosMsg = SendMsg | DelegateMsg | UndelegateMsg | RedelegateMsg | WithdrawMsg

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

interface CW1Instance {
  readonly contractAddress: string

  // queries
  admins: () => Promise<AdminListResponse>
  allowance: (address?: string) => Promise<AllowanceInfo>
  allAllowances: (startAfter?: string, limit?: number) => Promise<AllAllowancesResponse>

  permissions: (address?: string) => Promise<PermissionsInfo>
  allPermissions: (startAfter?: string, limit?: number) => Promise<AllPermissionsResponse>
  canExecute: (sender: string, msg: CosmosMsg) => Promise<CanExecuteResponse>

  // actions
  execute: (msgs: readonly CosmosMsg[]) => Promise<string>
  freeze: () => Promise<string>
  updateAdmins: (admins: readonly string[]) => Promise<string>
  increaseAllowance: (recipient: string, amount: Coin, expires?: Expiration) => Promise<string>
  decreaseAllowance: (recipient: string, amount: Coin, expires?: Expiration) => Promise<string>
  setPermissions: (recipient: string, permissions: Permissions) => Promise<string>
}

interface CW1Contract {
  // upload a code blob and returns a codeId
  upload: () => Promise<number>

  // instantiates a cw1-subkeys contract
  // codeId must come from a previous deploy
  // label is the public name of the contract in listing
  // if you set admin, you can run migrations on this contract (likely client.senderAddress)
  instantiate: (codeId: number, initMsg: Record<string, unknown>, label: string, admin?: string) => Promise<CW1Instance>

  use: (contractAddress: string) => CW1Instance
}


const CW1 = (client: SigningCosmWasmClient): CW1Contract => {
  const use = (contractAddress: string): CW1Instance => {
    const allowance = async (address?: string): Promise<AllowanceInfo> => {
      const spender = address || client.senderAddress;
      return await client.queryContractSmart(contractAddress, {allowance: {spender}});
    };

    const allAllowances = async (startAfter?: string, limit?: number): Promise<AllAllowancesResponse> => {
      return client.queryContractSmart(contractAddress, {all_allowances: { start_after: startAfter, limit: limit }});
    };

    const permissions = async (address?: string): Promise<PermissionsInfo> => {
      const spender = address || client.senderAddress;
      return await client.queryContractSmart(contractAddress, {permissions: {spender}});
    };

    const allPermissions = async (startAfter?: string, limit?: number): Promise<AllPermissionsResponse> => {
      return client.queryContractSmart(contractAddress, {all_permissions: { start_after: startAfter, limit: limit }});
    };

    const canExecute = async (sender: string, msg: CosmosMsg): Promise<CanExecuteResponse> => {
      return client.queryContractSmart(contractAddress, {can_execute: { sender: sender, msg: msg }});
    };

    const admins = async (): Promise<AdminListResponse> => {
      return client.queryContractSmart(contractAddress, {admin_list: { }});
    };

    // called by an admin to make admin set immutable
    const freeze = async (): Promise<string> => {
      const result = await client.execute(contractAddress, {freeze: {}});
      return result.transactionHash;
    }

    // burns tokens, returns transactionHash
    const updateAdmins = async (admins: readonly string[]): Promise<string> => {
      const result = await client.execute(contractAddress, {update_admins: {admins}});
      return result.transactionHash;
    }

    // transfers tokens, returns transactionHash
    const execute = async (msgs: readonly CosmosMsg[]): Promise<string> => {
      const result = await client.execute(contractAddress, {execute: {msgs}});
      return result.transactionHash;
    }

    const increaseAllowance = async (spender: string, amount: Coin, expires?: Expiration): Promise<string> => {
      const result = await client.execute(contractAddress, {increase_allowance: {spender, amount, expires}});
      return result.transactionHash;
    }

    const decreaseAllowance = async (spender: string, amount: Coin, expires?: Expiration): Promise<string> => {
      const result = await client.execute(contractAddress, {decrease_allowance: {spender, amount, expires}});
      return result.transactionHash;
    }

    const setPermissions = async (spender: string, permissions: Permissions): Promise<string> => {
      const result = await client.execute(contractAddress, {set_permissions: {spender, permissions}});
      return result.transactionHash;
    }

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
      setPermissions
    };
  }

  const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: 'arraybuffer' })
    if (r.status !== 200) {
      throw new Error(`Download error: ${r.status}`)
    }
    return r.data
  }
  
  const upload = async (): Promise<number> => {
    const meta = {
      source: "https://github.com/CosmWasm/cosmwasm-plus/tree/v0.4.0/contracts/cw1-subkeys",
      builder: "cosmwasm/rust-optimizer:0.10.7"
    };
    const sourceUrl = "https://github.com/CosmWasm/cosmwasm-plus/releases/download/v0.4.0/cw1_subkeys.wasm";
    const wasm = await downloadWasm(sourceUrl);
    const result = await client.upload(wasm, meta);
    return result.codeId;
  }

  const instantiate = async (codeId: number, initMsg: Record<string, unknown>, label: string, admin?: string): Promise<CW1Instance> => {
    const result = await client.instantiate(codeId, initMsg, label, { memo: `Init ${label}`, admin});
    return use(result.contractAddress);
  }

  return { upload, instantiate, use };
}

// Demo:
// const client = await useOptions(hackatomOptions).setup(PASSWORD);
// const { address } = await client.getAccount()
// const factory = CW1(client)
//
// const codeId = await factory.upload();
// codeId -> 12
// const contract = await factory.instantiate(12, { admins: [address], mutable: true}, "My Proxy")
// contract.contractAddress -> 'coral1267wq2zk22kt5juypdczw3k4wxhc4z47mug9fd'
// 
// OR
//
// const contract = factory.use('coral1267wq2zk22kt5juypdczw3k4wxhc4z47mug9fd')
//
// TODO: use a key you control to test out execute with subkey
// const randomAddress = 'coral162d3zk45ufaqke5wgcd3kh336k6p3kwwkdj3ma'
//
//
// contract.admins()
// contract.updateAdmins([address, randomAddress])
// contract.admins()
// -> remove this again so we can use subkeys
// contract.updateAdmins([address])
// contract.freeze()
// contract.admins()
//
// contract.allowance(randomAddress)
//
// contract.increaseAllowance(randomAddress, {denom: "ushell", amount: "123456"})
// contract.allowance(randomAddress)
// contract.increaseAllowance(randomAddress, {denom: "ureef", amount: "5000"})
// contract.decreaseAllowance(randomAddress, {denom: "ushell", amount: "3456"}, { at_height: { height: 500000 }})
// contract.allowance(randomAddress)
//
// -> send some tokens and then use the execute command
// const { contractAddress } = contract
// client.sendTokens(contractAddress, [{denom: "ushell", amount: "500000"}])
// client.getAccount(contractAddress)
// client.getAccount()
// TODO: send from randomAddress - some amount less than the allowance - same line will work
// contract.execute([{bank: {send: {from_address: contractAddress, to_address: address, amount: [{denom: "ushell", amount: "440000"}]}}}])
// client.getAccount(contractAddress)
// client.getAccount()

// let permissions: Permissions = { delegate: true, undelegate: true, redelegate: true, withdraw: true }
// contract.setStakingPermissions(randomAddress, permissions)

// test delegating and undelegating from another account
// let dmsg: DelegateMsg = {staking: {delegate: {validator:"coralvaloper1hf50trj7plz2sd8cmcvn7c8ruh3tjhc2uch4gp", amount:{denom:"ureef",amount:"999"}}}}
// contract.execute([dmsg])
//
// let unmsg: UndelegateMsg = {staking: {undelegate: {validator:"coralvaloper1hf50trj7plz2sd8cmcvn7c8ruh3tjhc2uch4gp", amount:{denom:"ureef",amount:"999"}}}}
// contract.execute([unmsg])

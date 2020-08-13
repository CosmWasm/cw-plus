/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * With these you can easily use the cw20 contract without worrying about forming messages and parsing queries.
 * 
 * Usage: npx @cosmjs/cli --init https://github.com/CosmWasm/cosmwasm-plus/blob/master/contracts/cw20-base/helpers.ts
 * 
 * Create a client:
 *   const client = await useOptions(coralnetOptions).setup(password);
 *   await client.getAccount()
 * 
 * Get the mnemonic:
 *   await useOptions(coralnetOptions).recoverMnemonic(password)
 * 
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */

const path = require("path");

interface Options {
  readonly httpUrl: string
  readonly networkId: string
  readonly feeToken: string
  readonly gasPrice: number
  readonly bech32prefix: string
  readonly hdPath: readonly Slip10RawIndex[]
  readonly faucetToken: string
  readonly faucetUrl?: string
  readonly defaultKeyFile: string
}
  
const coralnetOptions: Options = {
  httpUrl: 'https://lcd.coralnet.cosmwasm.com',
  networkId: 'cosmwasm-coral',
  feeToken: 'ushell',
  gasPrice: 0.025,
  bech32prefix: 'coral',
  faucetToken: 'SHELL',
  faucetUrl: 'https://faucet.coralnet.cosmwasm.com/credit',
  hdPath: makeCosmoshubPath(0),
  defaultKeyFile: path.join(process.env.HOME, ".coral.key"),
}

interface Network {
  setup: (password: string, filename?: string) => Promise<SigningCosmWasmClient>
  recoverMnemonic: (password: string, filename?: string) => Promise<string>
}

const useOptions = (options: Options): Network => {

  const loadOrCreateWallet = async (options: Options, filename: string, password: string): Promise<Secp256k1Wallet> => {
    try {
      const encrypted = fs.readFileSync(filename, 'utf8');
      const wallet = await Secp256k1Wallet.deserialize(encrypted, password);
      return wallet;
    } catch (err) {
      const wallet = await Secp256k1Wallet.generate(12, options.hdPath, options.bech32prefix);
      const encrypted = await wallet.serialize(password);
      fs.writeFileSync(filename, encrypted, 'utf8');
      return wallet;
    }
  };
  
  const buildFeeTable = (options: Options): FeeTable => {
    const { feeToken, gasPrice } = options;
    const stdFee = (gas: number, denom: string, price: number) => {
      const amount = Math.floor(gas * price)
      return {
        amount: [{ amount: amount.toString(), denom: denom }],
        gas: gas.toString(),
      }
    }
  
    return {
      upload: stdFee(1000000, feeToken, gasPrice),
      init: stdFee(500000, feeToken, gasPrice),
      migrate: stdFee(500000, feeToken, gasPrice),
      exec: stdFee(200000, feeToken, gasPrice),
      send: stdFee(80000, feeToken, gasPrice),
      changeAdmin: stdFee(80000, feeToken, gasPrice),
    }
  };

  const connect = async (
    wallet: Secp256k1Wallet,
    options: Options
  ): Promise<SigningCosmWasmClient> => {
    const feeTable = buildFeeTable(options);
    const [{ address }] = await wallet.getAccounts();
  
    const client = new SigningCosmWasmClient(
      options.httpUrl,
      address,
      wallet,
      feeTable
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
    const wallet = await loadOrCreateWallet(coralnetOptions, keyfile, password);
    const client = await connect(wallet, coralnetOptions);

    // ensure we have some tokens
    if (options.faucetUrl) {
      const account = await client.getAccount();
      if (!account) {
        console.log(`Getting ${options.feeToken} from faucet`);
        await hitFaucet(options.faucetUrl, client.senderAddress, options.faucetToken);
      }  
    }

    return client;
  }

  const recoverMnemonic = async (password: string, filename?: string): Promise<string> => {
    const keyfile = filename || options.defaultKeyFile;
    const wallet = await loadOrCreateWallet(coralnetOptions, keyfile, password);
    return wallet.mnemonic;
  }

  return {setup, recoverMnemonic};
}

interface Balances {
  readonly address: string
  readonly amount: string  // decimal as string
}

interface MintInfo {
  readonly minter: string
  readonly cap?: string // decimal as string
}

interface InitMsg {
  readonly name: string
  readonly symbol: string
  readonly decimals: number
  readonly initial_balances: readonly Balances[]
  readonly mint?: MintInfo
}

interface CW20Contract {
  // upload a code blob and returns a codeId
  upload: () => Promise<number>

  // instantiates a cw20 contract
  // codeId must come from a previous deploy
  // label is the public name of the contract in listing
  // if you set admin, you can run migrations on this contract (likely client.senderAddress)
  instantiate: (codeId: number, initMsg: InitMsg, label: string, admin?: string) => Promise<string>
}

const CW20 = (client: SigningCosmWasmClient): CW20Contract => {
  const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: 'arraybuffer' })
    if (r.status !== 200) {
      throw new Error(`Download error: ${r.status}`)
    }
    return r.data
  }
  
  const upload = async (): Promise<number> => {
    const meta = {
      source: "https://github.com/CosmWasm/cosmwasm-plus",
      builder: "cosmwasm/rust-optimizer:0.10.1`"
    };
    const sourceUrl = "https://github.com/CosmWasm/cosmwasm-plus/releases/download/v0.1.1/cw20_base.wasm";
    const wasm = await downloadWasm(sourceUrl);
    const result = await client.upload(wasm, meta);
    return result.codeId;
  }

  const instantiate = async (codeId: number, initMsg: InitMsg, label: string, admin?: string): Promise<string> => {
    const result = await client.instantiate(codeId, initMsg, label, { memo: `Init ${label}`, admin});
    return result.contractAddress;
  }

  return { upload, instantiate };
}


/*** this is demo code  ***/
const main = async () => {
  console.log("Running demo....");
  const client = await useOptions(coralnetOptions).setup("12345678");
  console.log(client.senderAddress);
  const account = await client.getAccount();
  console.log(account);

  const cw20 = CW20(client);
}

await main()

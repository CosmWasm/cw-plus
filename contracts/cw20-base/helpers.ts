/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * With these you can easily use the cw20 contract without worrying about forming messages and parsing queries.
 * 
 * Usage: npx @cosmjs/cli --init https://github.com/CosmWasm/cosmwasm-plus/blob/master/contracts/cw20-base/helpers.ts
 * 
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */

interface Options {
  readonly httpUrl: string
  readonly faucetUrl?: string
  readonly networkId: string
  readonly feeToken: string
  readonly gasPrice: number
  readonly bech32prefix: string
  readonly hdPath: readonly Slip10RawIndex[]
}
  
const coralnetOptions: Options = {
  httpUrl: 'https://lcd.coralnet.cosmwasm.com',
  networkId: 'cosmwasm-coral',
  feeToken: 'ushell',
  gasPrice: 0.025,
  bech32prefix: 'coral',
  faucetUrl: 'https://faucet.coralnet.cosmwasm.com/credit',
  hdPath: makeCosmoshubPath(0),
}

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
}

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
}

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
}

const downloadWasm = async (url: string): Promise<Uint8Array> => {
  const r = await axios.get(url, { responseType: 'arraybuffer' })
  if (r.status !== 200) {
    throw new Error(`Download error: ${r.status}`)
  }
  return r.data
}

const hitFaucet = async (
  faucetUrl: string,
  address: string,
  ticker: string
): Promise<void> => {
  const r = await axios.post(faucetUrl, { ticker, address });
  console.log(r.status);
  console.log(r.data);
}


/*** this is demo code  ***/
const main = async () => {
  console.log("Running demo....");
  const wallet = await loadOrCreateWallet(coralnetOptions, "demo.key", "12345678");
  const client = await connect(wallet, coralnetOptions);
  console.log(client.senderAddress);
  const account = await client.getAccount();
  console.log(account);
}

await main()

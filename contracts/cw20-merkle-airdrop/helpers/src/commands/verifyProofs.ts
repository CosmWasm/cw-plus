import {Command, flags} from '@oclif/command'
import { readFileSync } from 'fs';
import {Airdrop} from '../airdrop';

export default class VerifyProof extends Command {
  static description = 'Verifies merkle proofs for given address'

  static examples = [
    `$ PROOFS='[ "27e9b1ec8cb64709d0a8d3702344561674199fe81b885f1f9c9b2fb268795962","280777995d054081cbf208bccb70f8d736c1766b81d90a1fd21cd97d2d83a5cc","3946ea1758a5a2bf55bae1186168ad35aa0329805bc8bff1ca3d51345faec04a"]'
     $ merkle-airdrop-cli verifyProofs --file ../testdata/airdrop.json \
        --address wasm1k9hwzxs889jpvd7env8z49gad3a3633vg350tq \
        --amount 100
        --proofs $PROOFS
`,
  ]

  static flags = {
    help: flags.help({char: 'h'}),
    file: flags.string({char: 'f', description: 'airdrop file location'}),
    proofs: flags.string({char: 'p', description: 'proofs in json format'}),
    address: flags.string({char: 'a', description: 'address'}),
    amount: flags.string({char: 'b', description: 'amount'}),
  }

  async run() {
    const {flags} = this.parse(VerifyProof)

    if (!flags.file) {
      this.error(new Error('Airdrop file location not defined'))
    }
    if (!flags.proofs) {
      this.error(new Error('Proofs not defined'))
    }
    if (!flags.address) {
      this.error(new Error('Address not defined'))
    }
    if (!flags.amount) {
      this.error(new Error('Amount not defined'))
    }

    let file;
    try {
      file = readFileSync(flags.file, 'utf-8');
    } catch (e) {
      this.error(e)
    }

    let receivers: Array<{ address: string; amount: string }> = JSON.parse(file);

    let airdrop = new Airdrop(receivers)
    let proofs: string[] = JSON.parse(flags.proofs)

    console.log(airdrop.verify(proofs, {address: flags.address, amount: flags.amount}))
  }
}

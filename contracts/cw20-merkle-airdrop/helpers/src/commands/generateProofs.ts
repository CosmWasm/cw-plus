import {Command, flags} from '@oclif/command'
import { readFileSync } from 'fs';
import {Airdrop} from '../airdrop';

export default class GenerateProof extends Command {
  static description = 'Generates merkle proofs for given address'

  static examples = [
    `$ merkle-airdrop-cli generateProofs --file ../testdata/airdrop_stage_2.json \
        --address wasm1ylna88nach9sn5n7qe7u5l6lh7dmt6lp2y63xx \
        --amount 1000000000
`,
  ]

  static flags = {
    help: flags.help({char: 'h'}),
    file: flags.string({char: 'f', description: 'airdrop file location'}),
    address: flags.string({char: 'a', description: 'address'}),
    amount: flags.string({char: 'b', description: 'amount'}),
  }

  async run() {
    const {flags} = this.parse(GenerateProof)

    if (!flags.file) {
      this.error(new Error('Airdrop file location not defined'))
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
    let proof = airdrop.getMerkleProof({address: flags.address, amount: flags.amount})
    console.log(proof)
  }
}

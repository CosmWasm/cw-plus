import {Command, flags} from '@oclif/command'
import { readFileSync } from 'fs';
import {Airdrop} from '../airdrop';

export default class GenerateRoot extends Command {
  static description = 'Generates merkle root'

  static examples = [
    `$ merkle-airdrop-cli generate-root
hello world from ./src/hello.ts!
`,
  ]

  static flags = {
    help: flags.help({char: 'h'}),
    file: flags.string({char: 'f', description: 'balances file location'}),
  }

  async run() {
    const {flags} = this.parse(GenerateRoot)

    if (!flags.file) {
      this.error(new Error('balance file location not defined'))
    }

    let file;
    try {
      file = readFileSync(flags.file, 'utf-8');
    } catch (e) {
      this.error(e)
    }

    let receivers: Array<{ address: string; amount: string }> = JSON.parse(file);

    let airdrop = new Airdrop(receivers)
    console.log(airdrop.getMerkleRoot())
  }
}

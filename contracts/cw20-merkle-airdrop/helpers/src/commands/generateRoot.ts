import {Command, flags} from '@oclif/command'
import { readFileSync } from 'fs';
import * as path from 'path';
import * as helpers from '../helpers';
import { MerkleTree } from 'typescript-solidity-merkle-tree';
import { keccak256 } from 'ethereumjs-util';

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

    let receivers: helpers.AirdropReceiver[] = JSON.parse(file);
    let elements: Buffer[] = receivers.map(e => e.address.concat(e.balance)).map(e => Buffer.from(e))

    let tree = new MerkleTree(elements, keccak256);
    console.log(tree)
  }
}

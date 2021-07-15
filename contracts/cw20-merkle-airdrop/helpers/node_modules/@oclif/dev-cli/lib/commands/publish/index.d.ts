import { Command, flags } from '@oclif/command';
import * as Tarballs from '../../tarballs';
export default class Publish extends Command {
    static description: string;
    static flags: {
        root: flags.IOptionFlag<string>;
        targets: flags.IOptionFlag<string | undefined>;
    };
    buildConfig: Tarballs.IConfig;
    run(): Promise<void>;
}

import { Command, flags } from '@oclif/command';
export default class PackDeb extends Command {
    static description: string;
    static flags: {
        root: flags.IOptionFlag<string>;
    };
    run(): Promise<void>;
}

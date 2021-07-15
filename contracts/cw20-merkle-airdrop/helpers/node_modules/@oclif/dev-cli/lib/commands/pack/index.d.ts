import { Command, flags } from '@oclif/command';
export default class Pack extends Command {
    static description: string;
    static flags: {
        root: flags.IOptionFlag<string>;
        targets: flags.IOptionFlag<string | undefined>;
        xz: import("@oclif/parser/lib/flags").IBooleanFlag<boolean>;
    };
    run(): Promise<void>;
}

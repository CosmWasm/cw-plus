import { Command, flags } from '@oclif/command';
export default class PublishMacos extends Command {
    static description: string;
    static flags: {
        root: flags.IOptionFlag<string>;
    };
    run(): Promise<void>;
}

import { Command } from '@oclif/command';
export default class Publish extends Command {
    static description: string;
    static hidden: boolean;
    static flags: {};
    run(): Promise<void>;
}

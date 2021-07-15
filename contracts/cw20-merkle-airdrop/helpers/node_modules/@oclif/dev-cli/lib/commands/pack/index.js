"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const command_1 = require("@oclif/command");
const qq = require("qqjs");
const Tarballs = require("../../tarballs");
class Pack extends command_1.Command {
    async run() {
        const prevCwd = qq.cwd();
        if (process.platform === 'win32')
            throw new Error('pack does not function on windows');
        const { flags } = this.parse(Pack);
        const targets = flags.targets ? flags.targets.split(',') : undefined;
        const buildConfig = await Tarballs.buildConfig(flags.root, { xz: flags.xz, targets });
        await Tarballs.build(buildConfig);
        qq.cd(prevCwd);
    }
}
exports.default = Pack;
Pack.description = `packages oclif cli into tarballs

This can be used to create oclif CLIs that use the system node or that come preloaded with a node binary.
`;
Pack.flags = {
    root: command_1.flags.string({ char: 'r', description: 'path to oclif CLI root', default: '.', required: true }),
    targets: command_1.flags.string({ char: 't', description: 'comma-separated targets to pack (e.g.: linux-arm,win32-x64)' }),
    xz: command_1.flags.boolean({ description: 'also build xz', allowNo: true }),
};

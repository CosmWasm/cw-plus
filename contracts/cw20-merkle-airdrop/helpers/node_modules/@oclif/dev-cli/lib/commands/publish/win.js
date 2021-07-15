"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const command_1 = require("@oclif/command");
const qq = require("qqjs");
const aws_1 = require("../../aws");
const log_1 = require("../../log");
const Tarballs = require("../../tarballs");
class PublishWin extends command_1.Command {
    async run() {
        const { flags } = this.parse(PublishWin);
        const buildConfig = await Tarballs.buildConfig(flags.root);
        const { s3Config, version, config } = buildConfig;
        const S3Options = {
            Bucket: s3Config.bucket,
            ACL: s3Config.acl || 'public-read',
        };
        const root = buildConfig.channel === 'stable' ? '' : `channels/${buildConfig.channel}/`;
        const uploadWin = async (arch) => {
            const exe = buildConfig.dist(`win/${config.bin}-v${buildConfig.version}-${arch}.exe`);
            if (await qq.exists(exe))
                await aws_1.default.s3.uploadFile(exe, Object.assign(Object.assign({}, S3Options), { CacheControl: 'max-age=86400', Key: `${root}${config.bin}-${arch}.exe` }));
        };
        await uploadWin('x64');
        await uploadWin('x86');
        log_1.log(`published win ${version}`);
    }
}
exports.default = PublishWin;
PublishWin.description = 'publish windows installers built with pack:win';
PublishWin.flags = {
    root: command_1.flags.string({ char: 'r', description: 'path to oclif CLI root', default: '.', required: true }),
};

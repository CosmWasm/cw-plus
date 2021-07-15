"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const command_1 = require("@oclif/command");
const qq = require("qqjs");
const aws_1 = require("../../aws");
const log_1 = require("../../log");
const Tarballs = require("../../tarballs");
class PublishMacos extends command_1.Command {
    async run() {
        const { flags } = this.parse(PublishMacos);
        const buildConfig = await Tarballs.buildConfig(flags.root);
        const { s3Config, version, config } = buildConfig;
        const S3Options = {
            Bucket: s3Config.bucket,
            ACL: s3Config.acl || 'public-read',
        };
        const root = buildConfig.channel === 'stable' ? '' : `channels/${buildConfig.channel}/`;
        const pkg = buildConfig.dist(`macos/${config.bin}-v${buildConfig.version}.pkg`);
        if (await qq.exists(pkg))
            await aws_1.default.s3.uploadFile(pkg, Object.assign(Object.assign({}, S3Options), { CacheControl: 'max-age=86400', Key: `${root}${config.bin}.pkg` }));
        log_1.log(`published macos ${version}`);
    }
}
exports.default = PublishMacos;
PublishMacos.description = 'publish macos installers built with pack:macos';
PublishMacos.flags = {
    root: command_1.flags.string({ char: 'r', description: 'path to oclif CLI root', default: '.', required: true }),
};

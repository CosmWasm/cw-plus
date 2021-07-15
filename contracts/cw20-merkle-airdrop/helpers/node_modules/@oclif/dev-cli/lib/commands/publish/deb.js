"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const command_1 = require("@oclif/command");
const qq = require("qqjs");
const aws_1 = require("../../aws");
const log_1 = require("../../log");
const Tarballs = require("../../tarballs");
class PublishDeb extends command_1.Command {
    async run() {
        const { flags } = this.parse(PublishDeb);
        const buildConfig = await Tarballs.buildConfig(flags.root);
        const { s3Config, version, config } = buildConfig;
        const dist = (f) => buildConfig.dist(qq.join('deb', f));
        if (!await qq.exists(dist('Release')))
            this.error('run "oclif-dev pack:deb" before publishing');
        const S3Options = {
            Bucket: s3Config.bucket,
            ACL: s3Config.acl || 'public-read',
        };
        const remoteBase = buildConfig.channel === 'stable' ? 'apt' : `channels/${buildConfig.channel}/apt`;
        const upload = (file) => {
            return aws_1.default.s3.uploadFile(dist(file), Object.assign(Object.assign({}, S3Options), { CacheControl: 'max-age=86400', Key: [remoteBase, file].join('/') }));
        };
        const debVersion = `${buildConfig.version.split('-')[0]}-1`;
        const uploadDeb = async (arch) => {
            const deb = `${config.bin}_${debVersion}_${arch}.deb`;
            if (await qq.exists(dist(deb)))
                await upload(deb);
        };
        await uploadDeb('amd64');
        await uploadDeb('i386');
        await upload('Packages.gz');
        await upload('Packages.xz');
        await upload('Packages.bz2');
        await upload('Release');
        if (await qq.exists(dist('InRelease')))
            await upload('InRelease');
        if (await qq.exists(dist('Release.gpg')))
            await upload('Release.gpg');
        log_1.log(`published deb ${version}`);
    }
}
exports.default = PublishDeb;
PublishDeb.description = 'publish deb package built with pack:deb';
PublishDeb.flags = {
    root: command_1.flags.string({ char: 'r', description: 'path to oclif CLI root', default: '.', required: true }),
};

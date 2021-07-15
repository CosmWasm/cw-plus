"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const command_1 = require("@oclif/command");
const qq = require("qqjs");
const aws_1 = require("../../aws");
const log_1 = require("../../log");
const Tarballs = require("../../tarballs");
class Publish extends command_1.Command {
    async run() {
        const { flags } = this.parse(Publish);
        if (process.platform === 'win32')
            throw new Error('publish does not function on windows');
        const targetOpts = flags.targets ? flags.targets.split(',') : undefined;
        this.buildConfig = await Tarballs.buildConfig(flags.root, { targets: targetOpts });
        const { s3Config, targets, dist, version, config } = this.buildConfig;
        if (!await qq.exists(dist(config.s3Key('versioned', { ext: '.tar.gz' }))))
            this.error('run "oclif-dev pack" before publishing');
        const S3Options = {
            Bucket: s3Config.bucket,
            ACL: s3Config.acl || 'public-read',
        };
        // for (let target of targets) await this.uploadNodeBinary(target)
        const ManifestS3Options = Object.assign(Object.assign({}, S3Options), { CacheControl: 'max-age=86400', ContentType: 'application/json' });
        const uploadTarball = async (options) => {
            const TarballS3Options = Object.assign(Object.assign({}, S3Options), { CacheControl: 'max-age=604800' });
            const releaseTarballs = async (ext) => {
                const versioned = config.s3Key('versioned', ext, options);
                const unversioned = config.s3Key('unversioned', ext, options);
                await aws_1.default.s3.uploadFile(dist(versioned), Object.assign(Object.assign({}, TarballS3Options), { ContentType: 'application/gzip', Key: versioned }));
                await aws_1.default.s3.uploadFile(dist(versioned), Object.assign(Object.assign({}, TarballS3Options), { ContentType: 'application/gzip', Key: unversioned }));
            };
            await releaseTarballs('.tar.gz');
            if (this.buildConfig.xz)
                await releaseTarballs('.tar.xz');
            const manifest = config.s3Key('manifest', options);
            await aws_1.default.s3.uploadFile(dist(manifest), Object.assign(Object.assign({}, ManifestS3Options), { Key: manifest }));
        };
        if (targets.length > 0)
            log_1.log('uploading targets');
        // eslint-disable-next-line no-await-in-loop
        for (const target of targets)
            await uploadTarball(target);
        log_1.log('uploading vanilla');
        await uploadTarball();
        log_1.log(`published ${version}`);
    }
}
exports.default = Publish;
Publish.description = `publish an oclif CLI to S3

"aws-sdk" will need to be installed as a devDependency to publish.
`;
Publish.flags = {
    root: command_1.flags.string({ char: 'r', description: 'path to oclif CLI root', default: '.', required: true }),
    targets: command_1.flags.string({ char: 't', description: 'comma-separated targets to pack (e.g.: linux-arm,win32-x64)' }),
};

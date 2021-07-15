import * as Config from '@oclif/config';
export interface IConfig {
    root: string;
    gitSha: string;
    config: Config.IConfig;
    nodeVersion: string;
    version: string;
    tmp: string;
    updateConfig: IConfig['config']['pjson']['oclif']['update'];
    s3Config: IConfig['updateConfig']['s3'];
    channel: string;
    xz: boolean;
    targets: {
        platform: Config.PlatformTypes;
        arch: Config.ArchTypes;
    }[];
    workspace(target?: {
        platform: Config.PlatformTypes;
        arch: Config.ArchTypes;
    }): string;
    dist(input: string): string;
}
export interface IManifest {
    version: string;
    channel: string;
    gz: string;
    xz?: string;
    sha256gz: string;
    sha256xz?: string;
    baseDir: string;
    rollout?: number;
    node: {
        compatible: string;
        recommended: string;
    };
}
export declare function gitSha(cwd: string, options?: {
    short?: boolean;
}): Promise<string>;
export declare function buildConfig(root: string, options?: {
    xz?: boolean;
    targets?: string[];
}): Promise<IConfig>;

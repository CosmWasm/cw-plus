/// <reference types="node" />
export default class ProxyUtil {
    static env: NodeJS.ProcessEnv;
    static readonly httpProxy: string | undefined;
    static readonly httpsProxy: string | undefined;
    static readonly noProxy: string | undefined;
    static shouldDodgeProxy(host: string): boolean;
    static usingProxy(host?: string): boolean;
    static readonly sslCertDir: Array<string>;
    static readonly sslCertFile: Array<string>;
    static readonly certs: Array<Buffer>;
    static agent(https: boolean, host?: string): any;
}

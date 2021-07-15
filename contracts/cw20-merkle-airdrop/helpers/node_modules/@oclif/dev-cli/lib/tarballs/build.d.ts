import { IConfig } from './config';
export declare function build(c: IConfig, options?: {
    platform?: string;
    pack?: boolean;
}): Promise<void>;

import * as Tar from 'tar-fs';
export declare const tar: {
    gz: {
        pack(from: string | string[], to: string | string[], options?: Tar.PackOptions): Promise<unknown>;
    };
};

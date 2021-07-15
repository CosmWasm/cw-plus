import * as path from 'path';
export declare const home: string;
export { path };
/**
 * easier to use version of path.join()
 * flattens args so you can pass things in like join(['foo', 'bar']) or join('foo', 'bar')
 * the point of this is to make it so all the different qqjs tools can take in arrays as arguments to be joined
 * defaults to process.cwd()
 */
export declare function join(filepath?: string | string[]): string;
export declare function join(...filepath: string[]): string;
/**
 * cd into a directory
 */
export declare function cd(filepaths: string | string[]): void;
export declare function pushd(filepaths: string | string[]): void;
export declare function popd(): void;
export declare function cwd(): string;

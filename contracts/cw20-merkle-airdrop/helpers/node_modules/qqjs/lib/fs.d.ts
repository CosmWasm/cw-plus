import * as Globby from 'globby';
export declare type Filepath = string | string[];
/**
 * creates a directory if it does not exist
 * this will automatically join the paths if you pass multiple strings with path.join()
 */
export declare function mkdirp(...filepaths: (string | string[])[]): Promise<void>;
export declare namespace mkdirp {
    function sync(...filepaths: (string | string[])[]): void;
}
/**
 * glob matcher (find files)
 */
export declare function globby(patterns: string | string[], options?: Globby.GlobbyOptions): Promise<string[]>;
/**
 * output string to file
 * creates directory if not exists
 */
export declare function write(filepaths: string | string[], data: any, options?: {}): Promise<void>;
/**
 * read file into string
 */
export declare function read(filepaths: string | string[], options?: {}): Promise<string>;
/**
 * list files in directory
 */
export declare function ls(filepaths?: string | string[], options?: {
    fullpath?: boolean;
}): Promise<string[]>;
export declare function fileType(fp: string | string[]): Promise<'file' | 'directory' | 'symlink' | undefined>;
/**
 * copy files with fs.copy
 * can copy directories
 */
export declare function cp(source: string | string[], destinationpaths: string | string[], options?: {}): Promise<void>;
/**
 * rm -rf
 */
export declare function rm(...filesArray: (string | string[])[]): Promise<void>;
export declare function rmIfEmpty(...filesArray: (string | string[])[]): Promise<void>;
export declare function mv(source: string | string[], dest: string | string[]): Promise<void>;
export declare function exists(filepath: string | string[]): Promise<boolean>;
export declare namespace exists {
    function sync(filepath: string | string[]): boolean;
}
export declare function chmod(filepath: string | string[], mode: number): Promise<void>;
export declare function ln(from: Filepath, to: Filepath): Promise<void>;
/**
 * create a new temporary directory
 * uses tmp
 */
export declare function tmpDir(): Promise<string>;
export declare function emptyDir(filepath: string | string[]): Promise<void>;
export declare function hash(algo: string, fp: string | string[]): Promise<string>;

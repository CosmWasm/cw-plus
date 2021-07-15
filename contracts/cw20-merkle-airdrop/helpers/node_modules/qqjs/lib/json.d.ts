import * as WriteJSONFile from 'write-json-file';
/**
 * reads a json file in using load-json-file
 * this will automatically join the paths if you pass multiple strings with path.join()
 * can accept http urls
 */
export declare function readJSON(filepaths: string | string[]): Promise<any>;
/**
 * writes a json file with write-json-file
 * this will automatically join the paths if you pass an array of strings
 */
export declare function writeJSON(filepaths: string | string[], data: any, options?: WriteJSONFile.Options): Promise<void>;

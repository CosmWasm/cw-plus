/// <reference types="node" />
import * as Execa from 'execa';
/**
 * easy access to process.env
 */
export declare const env: NodeJS.ProcessEnv;
/**
 * run a command
 *
 * pass in a string to have it run with execa.shell(), or an file and array of strings for execa()
 */
export declare function x(cmd: string, options?: Execa.Options): Promise<Execa.ExecaReturns>;
export declare function x(cmd: string, args: string[], options?: Execa.Options): Promise<Execa.ExecaReturns>;
export declare namespace x {
    function exec(cmd: string, args: string[], options?: Execa.Options): Execa.ExecaChildProcess;
    function shell(cmd: string, options?: Execa.Options): Execa.ExecaChildProcess;
    function stdout(cmd: string, args?: string[], options?: Execa.Options): Promise<string>;
}

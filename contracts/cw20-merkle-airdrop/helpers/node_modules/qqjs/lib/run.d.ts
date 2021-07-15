export declare function run<T>(fn: (...args: any[]) => Promise<T>): Promise<T | void>;
export declare function handleError(err: Error): void;

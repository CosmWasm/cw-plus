declare namespace writeJsonFile {
	type Replacer = (this: unknown, key: string, value: any) => unknown;
	type SortKeys = (a: string, b: string) => number;

	interface Options {
		/**
		Indentation as a string or number of spaces. Pass in `undefined` for no formatting.

		@default '\t'
		*/
		readonly indent?: string | number | undefined;

		/**
		Detect indentation automatically if the file exists.

		@default false
		*/
		readonly detectIndent?: boolean;

		/**
		Sort the keys recursively. Optionally pass in a compare function.

		@default false
		*/
		readonly sortKeys?: boolean | SortKeys;

		/**
		Passed into `JSON.stringify`.
		*/
		readonly replacer?: Replacer | ReadonlyArray<number | string>;

		/**
		Mode used when writing the file.

		@default 0o666
		*/
		readonly mode?: number;
	}
}

declare const writeJsonFile: {
	/**
	Stringify and write JSON to a file atomically.

	Creates directories for you as needed.

	@example
	```
	import writeJsonFile = require('write-json-file');

	(async () => {
		await writeJsonFile('foo.json', {foo: true});
	})();
	```
	*/
	(
		filePath: string,
		data: unknown,
		options?: writeJsonFile.Options
	): Promise<void>;

	/**
	Stringify and write JSON to a file atomically.

	Creates directories for you as needed.

	@example
	```
	import writeJsonFile = require('write-json-file');

	writeJsonFile.sync('foo.json', {foo: true});
	```
	*/
	sync(
		filePath: string,
		data: unknown,
		options?: writeJsonFile.Options
	): void;
};

export = writeJsonFile;

# write-json-file [![Build Status](https://travis-ci.org/sindresorhus/write-json-file.svg?branch=master)](https://travis-ci.org/sindresorhus/write-json-file)

> Stringify and write JSON to a file [atomically](https://github.com/npm/write-file-atomic)

Creates directories for you as needed.

## Install

```
$ npm install write-json-file
```

## Usage

```js
const writeJsonFile = require('write-json-file');

(async () => {
	await writeJsonFile('foo.json', {foo: true});
})();
```

## API

### writeJsonFile(filePath, data, options?)

Returns a `Promise`.

### writeJsonFile.sync(filePath, data, options?)

#### options

Type: `object`

##### indent

Type: `string | number`\
Default: `'\t'`

Indentation as a string or number of spaces.

Pass in `undefined` for no formatting.

##### detectIndent

Type: `boolean`\
Default: `false`

Detect indentation automatically if the file exists.

##### sortKeys

Type: `boolean | Function`\
Default: `false`

Sort the keys recursively.

Optionally pass in a [`compare`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/sort) function.

##### replacer

Type: `Function`

Passed into [`JSON.stringify`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/JSON/stringify#The_replacer_parameter).

##### mode

Type: `number`\
Default: `0o666`

[Mode](https://en.wikipedia.org/wiki/File_system_permissions#Numeric_notation) used when writing the file.

## write-json-file for enterprise

Available as part of the Tidelift Subscription.

The maintainers of write-json-file and thousands of other packages are working with Tidelift to deliver commercial support and maintenance for the open source dependencies you use to build your applications. Save time, reduce risk, and improve code health, while paying the maintainers of the exact dependencies you use. [Learn more.](https://tidelift.com/subscription/pkg/npm-write-json-file?utm_source=npm-write-json-file&utm_medium=referral&utm_campaign=enterprise&utm_term=repo)

## Related

- [load-json-file](https://github.com/sindresorhus/load-json-file) - Read and parse a JSON file
- [make-dir](https://github.com/sindresorhus/make-dir) - Make a directory and its parents if needed


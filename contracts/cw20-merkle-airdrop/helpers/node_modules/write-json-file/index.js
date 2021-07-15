'use strict';
const {promisify} = require('util');
const path = require('path');
const fs = require('graceful-fs');
const writeFileAtomic = require('write-file-atomic');
const sortKeys = require('sort-keys');
const makeDir = require('make-dir');
const detectIndent = require('detect-indent');
const isPlainObj = require('is-plain-obj');

const readFile = promisify(fs.readFile);

const init = (fn, filePath, data, options) => {
	if (!filePath) {
		throw new TypeError('Expected a filepath');
	}

	if (data === undefined) {
		throw new TypeError('Expected data to stringify');
	}

	options = {
		indent: '\t',
		sortKeys: false,
		...options
	};

	if (options.sortKeys && isPlainObj(data)) {
		data = sortKeys(data, {
			deep: true,
			compare: typeof options.sortKeys === 'function' ? options.sortKeys : undefined
		});
	}

	return fn(filePath, data, options);
};

const main = async (filePath, data, options) => {
	let {indent} = options;
	let trailingNewline = '\n';
	try {
		const file = await readFile(filePath, 'utf8');
		if (!file.endsWith('\n')) {
			trailingNewline = '';
		}

		if (options.detectIndent) {
			indent = detectIndent(file).indent;
		}
	} catch (error) {
		if (error.code !== 'ENOENT') {
			throw error;
		}
	}

	const json = JSON.stringify(data, options.replacer, indent);

	return writeFileAtomic(filePath, `${json}${trailingNewline}`, {mode: options.mode, chown: false});
};

const mainSync = (filePath, data, options) => {
	let {indent} = options;
	let trailingNewline = '\n';
	try {
		const file = fs.readFileSync(filePath, 'utf8');
		if (!file.endsWith('\n')) {
			trailingNewline = '';
		}

		if (options.detectIndent) {
			indent = detectIndent(file).indent;
		}
	} catch (error) {
		if (error.code !== 'ENOENT') {
			throw error;
		}
	}

	const json = JSON.stringify(data, options.replacer, indent);

	return writeFileAtomic.sync(filePath, `${json}${trailingNewline}`, {mode: options.mode, chown: false});
};

module.exports = async (filePath, data, options) => {
	await makeDir(path.dirname(filePath), {fs});
	return init(main, filePath, data, options);
};

module.exports.sync = (filePath, data, options) => {
	makeDir.sync(path.dirname(filePath), {fs});
	init(mainSync, filePath, data, options);
};

'use strict';
const isPlainObject = require('is-plain-obj');

module.exports = (object, options = {}) => {
	if (!isPlainObject(object) && !Array.isArray(object)) {
		throw new TypeError('Expected a plain object or array');
	}

	const {deep} = options;
	const seenInput = [];
	const seenOutput = [];

	const deepSortArray = array => {
		const seenIndex = seenInput.indexOf(array);

		if (seenIndex !== -1) {
			return seenOutput[seenIndex];
		}

		const result = [];
		seenInput.push(array);
		seenOutput.push(result);

		result.push(...array.map(item => {
			if (Array.isArray(item)) {
				return deepSortArray(item);
			}

			if (isPlainObject(item)) {
				return sortKeys(item);
			}

			return item;
		}));

		return result;
	};

	const sortKeys = object => {
		const seenIndex = seenInput.indexOf(object);

		if (seenIndex !== -1) {
			return seenOutput[seenIndex];
		}

		const result = {};
		const keys = Object.keys(object).sort(options.compare);

		seenInput.push(object);
		seenOutput.push(result);

		for (const key of keys) {
			const value = object[key];
			let newValue;

			if (deep && Array.isArray(value)) {
				newValue = deepSortArray(value);
			} else {
				newValue = deep && isPlainObject(value) ? sortKeys(value) : value;
			}

			Object.defineProperty(result, key, {
				...Object.getOwnPropertyDescriptor(object, key),
				value: newValue
			});
		}

		return result;
	};

	if (Array.isArray(object)) {
		return deep ? deepSortArray(object) : object.slice();
	}

	return sortKeys(object);
};

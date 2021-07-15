# sort-keys [![Build Status](https://travis-ci.com/sindresorhus/sort-keys.svg?branch=master)](https://travis-ci.com/github/sindresorhus/sort-keys)

> Sort the keys of an object

Useful to get a deterministically ordered object, as the order of keys can vary between engines.

## Install

```
$ npm install sort-keys
```

## Usage

```js
const sortKeys = require('sort-keys');

sortKeys({c: 0, a: 0, b: 0});
//=> {a: 0, b: 0, c: 0}

sortKeys({b: {b: 0, a: 0}, a: 0}, {deep: true});
//=> {a: 0, b: {a: 0, b: 0}}

sortKeys({b: [{b: 0, a: 0}], a: 0}, {deep: true});
//=> {a: 0, b: [{a: 0, b: 0}]}

sortKeys({c: 0, a: 0, b: 0}, {
	compare: (a, b) => -a.localeCompare(b)
});
//=> {c: 0, b: 0, a: 0}

sortKeys([{b: 0, a:2}], {deep: true});
//=> [{a: 2, b: 0}]
```

## API

### sortKeys(object, options?)

Returns a new object with sorted keys.

#### object

Type: `object | Array`

#### options

Type: `object`

##### deep

Type: `boolean`\
Default: `false`

Recursively sort keys, including keys of objects inside arrays.

##### compare

Type: `Function`

[Compare function.](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/sort)

---

<div align="center">
	<b>
		<a href="https://tidelift.com/subscription/pkg/npm-sort-keys?utm_source=npm-sort-keys&utm_medium=referral&utm_campaign=readme">Get professional support for this package with a Tidelift subscription</a>
	</b>
	<br>
	<sub>
		Tidelift helps make open source sustainable for maintainers while giving companies<br>assurances about security, maintenance, and licensing for their dependencies.
	</sub>
</div>

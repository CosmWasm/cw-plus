# eslint-config-xo-space [![Build Status](https://travis-ci.org/xojs/eslint-config-xo-space.svg?branch=master)](https://travis-ci.org/xojs/eslint-config-xo-space)

> ESLint [shareable config](http://eslint.org/docs/developer-guide/shareable-configs.html) for [XO](https://github.com/xojs/xo) with 2-space indent

This is for advanced users. [You probably want to use XO directly.](https://github.com/xojs/eslint-config-xo#use-the-xo-cli-instead)


## Install

```
$ npm install --save-dev eslint-config-xo-space
```


## Usage

Add some ESLint config to your `package.json`:

```json
{
	"name": "my-awesome-project",
	"eslintConfig": {
		"extends": "xo-space"
	}
}
```

Or to `.eslintrc`:

```json
{
	"extends": "xo-space"
}
```

Supports parsing ES2015+, but doesn't enforce it by default.

This package also exposes [`xo-space/esnext`](esnext.js) if you want ES2015+ rules:

```json
{
	"extends": "xo-space/esnext"
}
```

And [`xo-space/browser`](browser.js) if you're in the browser:

```json
{
	"extends": "xo-space/browser"
}
```


## Related

- [eslint-config-xo](https://github.com/xojs/eslint-config-xo) - ESLint shareable config for XO
- [eslint-config-xo-react](https://github.com/xojs/eslint-config-xo-react) - ESLint shareable config for React to be used with the above


## License

MIT © [Sindre Sorhus](https://sindresorhus.com)

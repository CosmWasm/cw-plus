'use strict';
const path = require('path');

module.exports = {
	extends: path.join(__dirname, 'index.js'),
	rules: {
		'no-var': 'error',
		'object-shorthand': ['error', 'always'],
		'prefer-arrow-callback': ['error', {allowNamedFunctions: true}],
		'prefer-const': [
			'error',
			{
				destructuring: 'all'
			}
		],
		'prefer-numeric-literals': 'error',
		'prefer-rest-params': 'error',
		'prefer-spread': 'error',
		// TODO: Enable this when targeting Node.js 8
		// 'prefer-object-spread': 'error',
		'prefer-destructuring': [
			'error',
			{
				// `array` is disabled because it forces destructuring on
				// stupid stuff like `foo.bar = process.argv[2];`
				// TODO: Open ESLint issue about this
				VariableDeclarator: {
					array: false,
					object: true
				},
				AssignmentExpression: {
					array: false,

					// Disabled because object assignment destructuring requires parens wrapping:
					// `let foo; ({foo} = object);`
					object: false
				}
			},
			{
				enforceForRenamedProperties: false
			}
		]
	}
};

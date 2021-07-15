'use strict';

/**
 * @fileoverview Match test descriptions to match a pre-configured regular expression
 * @author Alexander Afanasyev
 */

const astUtils = require('../util/ast');

const defaultTestNames = [ 'it', 'test', 'specify' ];

module.exports = function (context) {
    const pattern = context.options[0] ? new RegExp(context.options[0]) : /^should/;
    const testNames = context.options[1] ? context.options[1] : defaultTestNames;

    function isTest(node) {
        return node.callee && node.callee.name && testNames.indexOf(node.callee.name) > -1;
    }

    function hasValidTestDescription(mochaCallExpression) {
        const args = mochaCallExpression.arguments;
        const testDescriptionArgument = args[0];

        if (astUtils.isStringLiteral(testDescriptionArgument)) {
            return pattern.test(testDescriptionArgument.value);
        }

        return true;
    }

    function hasValidOrNoTestDescription(mochaCallExpression) {
        const args = mochaCallExpression.arguments;
        const hasNoTestDescription = args.length === 0;

        return hasNoTestDescription || hasValidTestDescription(mochaCallExpression);
    }

    return {
        CallExpression(node) {
            const callee = node.callee;

            if (isTest(node)) {
                if (!hasValidOrNoTestDescription(node)) {
                    context.report(node, `Invalid "${ callee.name }()" description found.`);
                }
            }
        }
    };
};

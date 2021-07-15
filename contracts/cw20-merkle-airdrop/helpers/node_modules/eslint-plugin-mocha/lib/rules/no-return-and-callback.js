'use strict';

const R = require('ramda');
const astUtils = require('../util/ast');

const findReturnStatement = R.find(R.propEq('type', 'ReturnStatement'));

function hasParentMochaFunctionCall(functionExpression) {
    return astUtils.isTestCase(functionExpression.parent) || astUtils.isHookCall(functionExpression.parent);
}

function reportIfShortArrowFunction(context, node) {
    if (node.body.type !== 'BlockStatement') {
        context.report({
            node: node.body,
            message: 'Confusing implicit return in a test with callback'
        });
        return true;
    }
    return false;
}

function isExplicitUndefined(node) {
    return node && node.type === 'Identifier' && node.name === 'undefined';
}

function isReturnOfUndefined(node) {
    const argument = node.argument;
    const isImplicitUndefined = argument === null;

    return isImplicitUndefined || isExplicitUndefined(argument);
}

function isFunctionCallWithName(node, name) {
    return node.type === 'CallExpression' &&
        node.callee.type === 'Identifier' &&
        node.callee.name === name;
}

function isAllowedReturnStatement(node, doneName) {
    const argument = node.argument;

    if (isReturnOfUndefined(node) || argument.type === 'Literal') {
        return true;
    }

    return isFunctionCallWithName(argument, doneName);
}

function reportIfFunctionWithBlock(context, node, doneName) {
    const returnStatement = findReturnStatement(node.body.body);
    if (returnStatement && !isAllowedReturnStatement(returnStatement, doneName)) {
        context.report({
            node: returnStatement,
            message: 'Unexpected use of `return` in a test with callback'
        });
    }
}

module.exports = function (context) {
    function check(node) {
        if (node.params.length === 0 || !hasParentMochaFunctionCall(node)) {
            return;
        }

        if (!reportIfShortArrowFunction(context, node)) {
            reportIfFunctionWithBlock(context, node, node.params[0].name);
        }
    }

    return {
        FunctionExpression: check,
        ArrowFunctionExpression: check
    };
};

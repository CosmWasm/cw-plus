"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function run(fn) {
    return fn().catch(handleError);
}
exports.run = run;
function handleError(err) {
    console.error(err.stack);
    process.exitCode = 1;
}
exports.handleError = handleError;

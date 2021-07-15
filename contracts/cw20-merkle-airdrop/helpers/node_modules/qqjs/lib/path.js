"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const os = require("os");
const path = require("path");
exports.path = path;
const log_1 = require("./log");
exports.home = os.homedir();
const flatten = (arr) => arr.reduce((acc, val) => acc.concat(val), []);
function join(...filepath) {
    // tslint:disable-next-line strict-type-predicates
    if (typeof filepath[1] === 'number' && Array.isArray(filepath[2])) {
        // this is being called with .map()
        filepath = [filepath[0]];
    }
    if (!filepath.length)
        return process.cwd();
    return path.join(...flatten(filepath));
}
exports.join = join;
/**
 * cd into a directory
 */
function cd(filepaths) {
    const filepath = join(filepaths);
    if (filepath === process.cwd())
        return; // don't log if no change
    log_1.log('cd', filepath);
    return process.chdir(filepath);
}
exports.cd = cd;
const origPath = process.cwd();
const pushdPaths = [];
function pushd(filepaths) {
    const f = join(filepaths);
    log_1.log('pushd', f);
    pushdPaths.push(process.cwd());
    return process.chdir(f);
}
exports.pushd = pushd;
function popd() {
    const f = pushdPaths.pop() || origPath;
    log_1.log('popd', f);
    return process.chdir(f);
}
exports.popd = popd;
function cwd() {
    const cwd = process.cwd();
    log_1.log('cwd', cwd);
    return cwd;
}
exports.cwd = cwd;

"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const os = require("os");
const config = require("./config");
exports.debug = require('debug')('qq');
const m = {
    m: {},
    get chalk() { return this.m.chalk = this.m.chalk || require('chalk'); },
};
const homeRegexp = new RegExp(`\\B${os.homedir().replace('/', '\\/')}`, 'g');
const curRegexp = new RegExp(`\\B${process.cwd()}`, 'g');
function log(...args) {
    const output = args.map(exports.prettifyPaths).join(' ');
    exports.debug(output);
    if (exports.debug.enabled || config.silent)
        return;
    console.log(`${m.chalk.gray('qq')} ${output}`);
}
exports.log = log;
exports.prettifyPaths = (input) => (input || '').toString().replace(curRegexp, '.').replace(homeRegexp, '~');

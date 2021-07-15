"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const log_1 = require("./log");
const m = {
    m: {},
    get execa() { return this.m.execa = this.m.execa || require('execa'); },
};
/**
 * easy access to process.env
 */
exports.env = process.env;
async function x(cmd, args, options = {}) {
    if (Array.isArray(args))
        return x.exec(cmd, args, options);
    else
        return x.shell(cmd, args);
}
exports.x = x;
(function (x) {
    function exec(cmd, args, options = {}) {
        options = Object.assign({ stdio: 'inherit' }, options);
        log_1.log('$', cmd, ...args);
        return m.execa(cmd, args, options);
    }
    x.exec = exec;
    function shell(cmd, options = {}) {
        options = Object.assign({ stdio: 'inherit' }, options);
        log_1.log('$', cmd);
        return m.execa.shell(cmd, options);
    }
    x.shell = shell;
    async function stdout(cmd, args = [], options = {}) {
        const getStream = require('get-stream');
        options = Object.assign({ stdio: [0, 'pipe', 2] }, options);
        log_1.log('$', cmd, ...args);
        const ps = m.execa(cmd, args, options);
        return new Promise((resolve, reject) => {
            ps.on('error', reject);
            if (log_1.debug.enabled && ps.stdout)
                ps.stdout.pipe(process.stdout);
            Promise.all([
                getStream(ps.stdout).then((o) => o.replace(/\n$/, '')),
                ps,
            ])
                .then(([stdout]) => resolve(stdout))
                .catch(reject);
        });
    }
    x.stdout = stdout;
})(x = exports.x || (exports.x = {}));

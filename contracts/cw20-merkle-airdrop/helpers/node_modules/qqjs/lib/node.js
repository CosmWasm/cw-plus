"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const log_1 = require("./log");
const path_1 = require("./path");
let _pkgDir;
const init = () => _pkgDir = _pkgDir || require('pkg-dir');
function pkgDir(fp) {
    init();
    const f = path_1.join(fp || process.cwd());
    log_1.log('pkgDir', f);
    return _pkgDir(f);
}
exports.pkgDir = pkgDir;
(function (pkgDir) {
    function sync(fp) {
        init();
        const f = path_1.join(fp || process.cwd());
        log_1.log('pkgDir.sync', f);
        return _pkgDir.sync(f);
    }
    pkgDir.sync = sync;
})(pkgDir = exports.pkgDir || (exports.pkgDir = {}));

"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const fs = require("fs-extra");
const path = require("path");
const exec_1 = require("./exec");
const log_1 = require("./log");
const path_1 = require("./path");
async function download(url, filepath) {
    filepath = filepath ? path_1.join(filepath) : path.basename(url);
    log_1.log('download', url, filepath);
    await fs.mkdirp(path.dirname(filepath));
    return exec_1.x('curl', ['-fsSLo', filepath, url]);
}
exports.download = download;

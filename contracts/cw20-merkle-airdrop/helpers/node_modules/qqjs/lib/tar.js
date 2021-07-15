"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const fs = require("fs-extra");
const path_1 = require("./path");
const deps = {
    m: {},
    get tar() { return this.m.tar = this.m.tar || require('tar-fs'); },
};
exports.tar = {
    gz: {
        pack(from, to, options = {}) {
            const _from = path_1.join(from);
            const _to = path_1.join(to);
            return new Promise((resolve, reject) => {
                deps.tar
                    .pack(_from, options)
                    .pipe(fs.createWriteStream(_to))
                    .on('error', reject)
                    .on('finish', resolve);
            });
        }
    }
};

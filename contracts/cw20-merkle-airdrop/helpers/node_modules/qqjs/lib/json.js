"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const log_1 = require("./log");
const path_1 = require("./path");
const deps = {
    m: {},
    get HTTP() { return this.m.HTTP = this.m.HTTP || require('http-call').default; },
    get loadJSONFile() { return this.m.loadJSONFile = this.m.loadJSONFile || require('load-json-file'); },
    get writeJSONFile() { return this.m.writeJSONFile = this.m.writeJSONFile || require('write-json-file'); },
};
/**
 * reads a json file in using load-json-file
 * this will automatically join the paths if you pass multiple strings with path.join()
 * can accept http urls
 */
function readJSON(filepaths) {
    async function readJSONHTTP(url) {
        const { body } = await deps.HTTP.get(url);
        return body;
    }
    if (typeof filepaths === 'string' && filepaths.match(/https?:/))
        return readJSONHTTP(filepaths);
    const filepath = path_1.join(filepaths);
    log_1.log('readJSON', filepath);
    return deps.loadJSONFile(filepath);
}
exports.readJSON = readJSON;
/**
 * writes a json file with write-json-file
 * this will automatically join the paths if you pass an array of strings
 */
function writeJSON(filepaths, data, options = {}) {
    const filepath = path_1.join(filepaths);
    log_1.log('writeJSON', filepath);
    return deps.writeJSONFile(filepath, data, Object.assign({ indent: '  ' }, options));
}
exports.writeJSON = writeJSON;

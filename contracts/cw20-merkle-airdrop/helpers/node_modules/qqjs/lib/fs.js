"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const crypto = require("crypto");
const fs = require("fs-extra");
const path = require("path");
const util_1 = require("util");
const log_1 = require("./log");
const path_1 = require("./path");
const deps = {
    m: {},
    get tmp() { return this.m.tmp = this.m.globby || require('tmp'); },
    get globby() { return this.m.globby = this.m.globby || require('globby'); },
};
/**
 * creates a directory if it does not exist
 * this will automatically join the paths if you pass multiple strings with path.join()
 */
async function mkdirp(...filepaths) {
    for (let f of filepaths.map(path_1.join)) {
        log_1.log('mkdirp', f);
        await fs.mkdirp(f);
    }
}
exports.mkdirp = mkdirp;
(function (mkdirp) {
    function sync(...filepaths) {
        for (let f of filepaths.map(path_1.join)) {
            log_1.log('mkdirpSync', f);
            fs.mkdirpSync(f);
        }
    }
    mkdirp.sync = sync;
})(mkdirp = exports.mkdirp || (exports.mkdirp = {}));
/**
 * glob matcher (find files)
 */
function globby(patterns, options = {}) {
    log_1.log('globby', ...patterns);
    return deps.globby(patterns, options);
}
exports.globby = globby;
/**
 * output string to file
 * creates directory if not exists
 */
function write(filepaths, data, options = {}) {
    const filepath = path_1.join(filepaths);
    log_1.log('write', filepath);
    return fs.outputFile(filepath, data, options);
}
exports.write = write;
/**
 * read file into string
 */
function read(filepaths, options = {}) {
    const filepath = path_1.join(filepaths);
    log_1.log('read', filepath);
    return fs.readFile(filepath, Object.assign({ encoding: 'utf8' }, options));
}
exports.read = read;
/**
 * list files in directory
 */
async function ls(filepaths, options = {}) {
    const filepath = path_1.join(filepaths);
    // log('ls', filepath)
    const files = await fs.readdir(filepath);
    if (options.fullpath)
        return files.map(f => path_1.join([filepath, f]));
    else
        return files;
}
exports.ls = ls;
async function fileType(fp) {
    try {
        const stats = await fs.stat(path_1.join(fp));
        if (stats.isSymbolicLink())
            return 'symlink';
        if (stats.isDirectory())
            return 'directory';
        if (stats.isFile())
            return 'file';
    }
    catch (err) {
        if (err.code === 'ENOENT')
            return;
        throw err;
    }
}
exports.fileType = fileType;
/**
 * copy files with fs.copy
 * can copy directories
 */
async function cp(source, destinationpaths, options = {}) {
    source = path_1.join(source);
    let dest = path_1.join(destinationpaths);
    switch (await fileType(dest)) {
        case 'directory':
            dest = path.join(dest, path.basename(source));
            break;
        case 'file':
            await rm(dest);
    }
    log_1.log('cp', source, dest);
    return fs.copy(source, dest, options);
}
exports.cp = cp;
/**
 * rm -rf
 */
async function rm(...filesArray) {
    for (let f of filesArray.map(path_1.join)) {
        log_1.log('rm', f);
        await fs.remove(f);
    }
}
exports.rm = rm;
async function rmIfEmpty(...filesArray) {
    const rmdir = async (f) => {
        let removedSomething = false;
        const getFiles = async () => (await ls(f)).map(s => path_1.join([f, s]));
        let files = await getFiles();
        for (let subdir of files) {
            if ((await fileType(subdir)) === 'directory') {
                await rmdir(subdir);
                removedSomething = true;
            }
        }
        // check files again if we removed any
        if (removedSomething)
            files = await getFiles();
        if (files.length === 0)
            await rm(f);
    };
    for (let f of filesArray.map(path_1.join)) {
        log_1.log('rmIfEmpty', f);
        await rmdir(f);
    }
}
exports.rmIfEmpty = rmIfEmpty;
async function mv(source, dest) {
    source = path_1.join(source);
    dest = path_1.join(dest);
    switch (await fileType(dest)) {
        case 'directory':
            dest = path.join(dest, path.basename(source));
            break;
        case 'file':
            await rm(dest);
    }
    log_1.log('mv', source, dest);
    return fs.move(source, dest);
}
exports.mv = mv;
async function exists(filepath) {
    filepath = path_1.join(filepath);
    const exists = await fs.pathExists(filepath);
    log_1.log('exists', filepath, exists);
    return exists;
}
exports.exists = exists;
(function (exists_1) {
    function sync(filepath) {
        filepath = path_1.join(filepath);
        const exists = fs.pathExistsSync(filepath);
        log_1.log('exists.sync', filepath, exists);
        return exists;
    }
    exists_1.sync = sync;
})(exists = exports.exists || (exports.exists = {}));
function chmod(filepath, mode) {
    filepath = path_1.join(filepath);
    log_1.log('chmod', filepath, mode.toString(8));
    return fs.chmod(filepath, mode);
}
exports.chmod = chmod;
function ln(from, to) {
    from = path_1.join(from);
    to = path_1.join(to);
    log_1.log('ln', from, to);
    return fs.link(from, to);
}
exports.ln = ln;
/**
 * create a new temporary directory
 * uses tmp
 */
async function tmpDir() {
    const output = await util_1.promisify(deps.tmp.dir)();
    log_1.log('tmpDir', output.name);
    return output.name;
}
exports.tmpDir = tmpDir;
async function emptyDir(filepath) {
    filepath = path_1.join(filepath);
    log_1.log('emptyDir', filepath);
    await fs.mkdirp(path.dirname(filepath));
    return fs.emptyDir(filepath);
}
exports.emptyDir = emptyDir;
async function hash(algo, fp) {
    const f = path_1.join(fp);
    log_1.log('hash', algo, f);
    return new Promise((resolve, reject) => {
        const hash = crypto.createHash(algo);
        const stream = fs.createReadStream(f);
        stream.on('error', err => reject(err));
        stream.on('data', chunk => hash.update(chunk));
        stream.on('end', () => resolve(hash.digest('hex')));
    });
}
exports.hash = hash;

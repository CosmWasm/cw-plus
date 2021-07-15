"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.sha512 = exports.Sha512 = exports.sha256 = exports.Sha256 = exports.sha1 = exports.Sha1 = void 0;
const sha_js_1 = __importDefault(require("sha.js"));
class Sha1 {
    constructor(firstData) {
        this.blockSize = 512 / 8;
        this.impl = sha_js_1.default("sha1");
        if (firstData) {
            this.update(firstData);
        }
    }
    update(data) {
        this.impl.update(data);
        return this;
    }
    digest() {
        return new Uint8Array(this.impl.digest());
    }
}
exports.Sha1 = Sha1;
/** Convenience function equivalent to `new Sha1(data).digest()` */
function sha1(data) {
    return new Sha1(data).digest();
}
exports.sha1 = sha1;
class Sha256 {
    constructor(firstData) {
        this.blockSize = 512 / 8;
        this.impl = sha_js_1.default("sha256");
        if (firstData) {
            this.update(firstData);
        }
    }
    update(data) {
        this.impl.update(data);
        return this;
    }
    digest() {
        return new Uint8Array(this.impl.digest());
    }
}
exports.Sha256 = Sha256;
/** Convenience function equivalent to `new Sha256(data).digest()` */
function sha256(data) {
    return new Sha256(data).digest();
}
exports.sha256 = sha256;
class Sha512 {
    constructor(firstData) {
        this.blockSize = 1024 / 8;
        this.impl = sha_js_1.default("sha512");
        if (firstData) {
            this.update(firstData);
        }
    }
    update(data) {
        this.impl.update(data);
        return this;
    }
    digest() {
        return new Uint8Array(this.impl.digest());
    }
}
exports.Sha512 = Sha512;
/** Convenience function equivalent to `new Sha512(data).digest()` */
function sha512(data) {
    return new Sha512(data).digest();
}
exports.sha512 = sha512;
//# sourceMappingURL=sha.js.map
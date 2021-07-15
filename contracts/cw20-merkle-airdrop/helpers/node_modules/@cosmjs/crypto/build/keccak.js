"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.keccak256 = exports.Keccak256 = void 0;
const js_sha3_1 = __importDefault(require("js-sha3"));
class Keccak256 {
    constructor(firstData) {
        this.blockSize = 512 / 8;
        this.impl = js_sha3_1.default.keccak256.create();
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
exports.Keccak256 = Keccak256;
/** Convenience function equivalent to `new Keccak256(data).digest()` */
function keccak256(data) {
    return new Keccak256(data).digest();
}
exports.keccak256 = keccak256;
//# sourceMappingURL=keccak.js.map
"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.ripemd160 = exports.Ripemd160 = void 0;
const ripemd160_1 = __importDefault(require("ripemd160"));
class Ripemd160 {
    constructor(firstData) {
        this.blockSize = 512 / 8;
        this.impl = new ripemd160_1.default();
        if (firstData) {
            this.update(firstData);
        }
    }
    update(data) {
        this.impl.update(Buffer.from(data));
        return this;
    }
    digest() {
        return Uint8Array.from(this.impl.digest());
    }
}
exports.Ripemd160 = Ripemd160;
/** Convenience function equivalent to `new Ripemd160(data).digest()` */
function ripemd160(data) {
    return new Ripemd160(data).digest();
}
exports.ripemd160 = ripemd160;
//# sourceMappingURL=ripemd.js.map
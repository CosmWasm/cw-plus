"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    Object.defineProperty(o, k2, { enumerable: true, get: function() { return m[k]; } });
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Bech32 = void 0;
const bech32 = __importStar(require("bech32"));
class Bech32 {
    static encode(prefix, data, limit) {
        const address = bech32.encode(prefix, bech32.toWords(data), limit);
        return address;
    }
    static decode(address, limit = Infinity) {
        const decodedAddress = bech32.decode(address, limit);
        return {
            prefix: decodedAddress.prefix,
            data: new Uint8Array(bech32.fromWords(decodedAddress.words)),
        };
    }
}
exports.Bech32 = Bech32;
//# sourceMappingURL=bech32.js.map
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
exports.Bip39 = void 0;
const encoding_1 = require("@cosmjs/encoding");
const bip39 = __importStar(require("bip39"));
const englishmnemonic_1 = require("./englishmnemonic");
class Bip39 {
    /**
     * Encodes raw entropy of length 16, 20, 24, 28 or 32 bytes as an English mnemonic between 12 and 24 words.
     *
     * | Entropy            | Words |
     * |--------------------|-------|
     * | 128 bit (16 bytes) |    12 |
     * | 160 bit (20 bytes) |    15 |
     * | 192 bit (24 bytes) |    18 |
     * | 224 bit (28 bytes) |    21 |
     * | 256 bit (32 bytes) |    24 |
     *
     *
     * @see https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki#generating-the-mnemonic
     * @param entropy The entropy to be encoded. This must be cryptographically secure.
     */
    static encode(entropy) {
        const allowedEntropyLengths = [16, 20, 24, 28, 32];
        if (allowedEntropyLengths.indexOf(entropy.length) === -1) {
            throw new Error("invalid input length");
        }
        return new englishmnemonic_1.EnglishMnemonic(bip39.entropyToMnemonic(encoding_1.toHex(entropy)));
    }
    static decode(mnemonic) {
        return encoding_1.fromHex(bip39.mnemonicToEntropy(mnemonic.toString()));
    }
    static async mnemonicToSeed(mnemonic, password) {
        return new Uint8Array(await bip39.mnemonicToSeed(mnemonic.toString(), password));
    }
}
exports.Bip39 = Bip39;
//# sourceMappingURL=bip39.js.map
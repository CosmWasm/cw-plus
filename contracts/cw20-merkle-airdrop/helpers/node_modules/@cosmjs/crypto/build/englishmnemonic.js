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
exports.EnglishMnemonic = void 0;
const bip39 = __importStar(require("bip39"));
class EnglishMnemonic {
    constructor(mnemonic) {
        if (!EnglishMnemonic.mnemonicMatcher.test(mnemonic)) {
            throw new Error("Invalid mnemonic format");
        }
        const words = mnemonic.split(" ");
        const allowedWordsLengths = [12, 15, 18, 21, 24];
        if (allowedWordsLengths.indexOf(words.length) === -1) {
            throw new Error(`Invalid word count in mnemonic (allowed: ${allowedWordsLengths} got: ${words.length})`);
        }
        for (const word of words) {
            if (EnglishMnemonic.wordlist.indexOf(word) === -1) {
                throw new Error("Mnemonic contains invalid word");
            }
        }
        // Throws with informative error message if mnemonic is not valid
        bip39.mnemonicToEntropy(mnemonic);
        this.data = mnemonic;
    }
    toString() {
        return this.data;
    }
}
exports.EnglishMnemonic = EnglishMnemonic;
EnglishMnemonic.wordlist = bip39.wordlists.english;
// list of space separated lower case words (1 or more)
EnglishMnemonic.mnemonicMatcher = /^[a-z]+( [a-z]+)*$/;
//# sourceMappingURL=englishmnemonic.js.map
"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const encoding_1 = require("@cosmjs/encoding");
const englishmnemonic_1 = require("./englishmnemonic");
const sha_1 = require("./sha");
const bip39_wordlists_json_1 = __importDefault(require("./testdata/bip39_wordlists.json"));
describe("EnglishMnemonic", () => {
    describe("wordlist", () => {
        it("matches the words from the bitcoin/bips/bip-0039 spec", () => {
            const lineFeed = 0x0a;
            const bip39EnglishTxt = encoding_1.fromBase64(bip39_wordlists_json_1.default.english);
            // Ensure we loaded the correct english.txt from https://github.com/bitcoin/bips/tree/master/bip-0039
            const checksum = sha_1.sha256(bip39EnglishTxt);
            expect(checksum).toEqual(encoding_1.fromHex("2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda"));
            const wordsFromSpec = [];
            let start = 0; // the start cursor marks the first byte of the word
            let end = 0; // the end cursor marks the line feed byte
            while (end < bip39EnglishTxt.length - 1) {
                end = start;
                while (bip39EnglishTxt[end] !== lineFeed)
                    end++;
                const slice = bip39EnglishTxt.slice(start, end);
                wordsFromSpec.push(encoding_1.fromAscii(slice));
                start = end + 1;
            }
            expect(englishmnemonic_1.EnglishMnemonic.wordlist).toEqual(wordsFromSpec);
        });
    });
    it("works for valid inputs", () => {
        expect(() => {
            new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about");
            new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon address");
            new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent");
            new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon admit");
            new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art");
        }).not.toThrow();
    });
    it("rejects invalid whitespacing", () => {
        // extra space (leading, middle, trailing)
        expect(() => new englishmnemonic_1.EnglishMnemonic(" abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon  abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about ")).toThrowError(/invalid mnemonic format/i);
        // newline, tab
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon\nabandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon\tabandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
    });
    it("rejects disallowed letters", () => {
        // Disallowed letters in words (capital, number, special char)
        expect(() => new englishmnemonic_1.EnglishMnemonic("Abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon Abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("route66 abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon route66 abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("lötkolben abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon lötkolben abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid mnemonic format/i);
    });
    it("word counts other than 12, 15, 18, 21, 24", () => {
        // too few and too many words (11, 13, 17, 19, 23, 25)
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid word count(.*)got: 11/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")).toThrowError(/invalid word count(.*)got: 13/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent")).toThrowError(/invalid word count(.*)got: 17/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent")).toThrowError(/invalid word count(.*)got: 19/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art")).toThrowError(/invalid word count(.*)got: 23/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art")).toThrowError(/invalid word count(.*)got: 25/i);
    });
    it("rejects invalid checksums", () => {
        // 12x, 15x, 18x, 21x, 24x "zoo"
        expect(() => new englishmnemonic_1.EnglishMnemonic("zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo")).toThrowError(/invalid mnemonic checksum/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo")).toThrowError(/invalid mnemonic checksum/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo")).toThrowError(/invalid mnemonic checksum/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo")).toThrowError(/invalid mnemonic checksum/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo")).toThrowError(/invalid mnemonic checksum/i);
    });
    it("rejects valid mnemonics of other languages", () => {
        // valid Spanish and Italian bip39 mnemonics
        expect(() => new englishmnemonic_1.EnglishMnemonic("humo odio oriente colina taco fingir salto geranio glaciar academia suave vigor")).toThrowError(/contains invalid word/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("yema folleto tos llave obtener natural fruta deseo laico sopa novato lazo imponer afinar vena hoja zarza cama")).toThrowError(/contains invalid word/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("burla plaza arroz ronda pregunta vacuna veloz boina retiro exento prensa tortuga cabeza pilar anual molino molde fiesta masivo jefe leve fatiga clase plomo")).toThrowError(/contains invalid word/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("braccio trincea armonia emiro svedese lepre stridulo metallo baldo rasente potassio rilassato")).toThrowError(/contains invalid word/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("riparato arrosto globulo singolo bozzolo roba pirolisi ultimato padrone munto leggero avanzato monetario guanto lorenzo latino inoltrare modulo")).toThrowError(/contains invalid word/i);
        expect(() => new englishmnemonic_1.EnglishMnemonic("promessa mercurio spessore snodo trave risata mecenate vichingo ceto orecchino vissuto risultato canino scarso futile fune epilogo uovo inedito apatico folata egoismo rifugio coma")).toThrowError(/contains invalid word/i);
    });
    describe("toString", () => {
        it("works", () => {
            const original = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
            const mnemonic = new englishmnemonic_1.EnglishMnemonic(original);
            expect(mnemonic.toString()).toEqual(original);
        });
    });
});
//# sourceMappingURL=englishmnemonic.spec.js.map
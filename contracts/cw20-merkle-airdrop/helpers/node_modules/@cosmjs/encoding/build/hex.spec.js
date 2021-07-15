"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const hex_1 = require("./hex");
describe("fromHex", () => {
    it("works", () => {
        // simple
        expect(hex_1.fromHex("")).toEqual(new Uint8Array([]));
        expect(hex_1.fromHex("00")).toEqual(new Uint8Array([0x00]));
        expect(hex_1.fromHex("01")).toEqual(new Uint8Array([0x01]));
        expect(hex_1.fromHex("10")).toEqual(new Uint8Array([0x10]));
        expect(hex_1.fromHex("11")).toEqual(new Uint8Array([0x11]));
        expect(hex_1.fromHex("112233")).toEqual(new Uint8Array([0x11, 0x22, 0x33]));
        expect(hex_1.fromHex("0123456789abcdef")).toEqual(new Uint8Array([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]));
        // capital letters
        expect(hex_1.fromHex("AA")).toEqual(new Uint8Array([0xaa]));
        expect(hex_1.fromHex("aAbBcCdDeEfF")).toEqual(new Uint8Array([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));
        // error
        expect(() => hex_1.fromHex("a")).toThrow();
        expect(() => hex_1.fromHex("aaa")).toThrow();
        expect(() => hex_1.fromHex("a!")).toThrow();
        expect(() => hex_1.fromHex("a ")).toThrow();
        expect(() => hex_1.fromHex("aa ")).toThrow();
        expect(() => hex_1.fromHex(" aa")).toThrow();
        expect(() => hex_1.fromHex("a a")).toThrow();
        expect(() => hex_1.fromHex("gg")).toThrow();
    });
});
describe("toHex", () => {
    it("works", () => {
        expect(hex_1.toHex(new Uint8Array([]))).toEqual("");
        expect(hex_1.toHex(new Uint8Array([0x00]))).toEqual("00");
        expect(hex_1.toHex(new Uint8Array([0x01]))).toEqual("01");
        expect(hex_1.toHex(new Uint8Array([0x10]))).toEqual("10");
        expect(hex_1.toHex(new Uint8Array([0x11]))).toEqual("11");
        expect(hex_1.toHex(new Uint8Array([0x11, 0x22, 0x33]))).toEqual("112233");
        expect(hex_1.toHex(new Uint8Array([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]))).toEqual("0123456789abcdef");
    });
});
//# sourceMappingURL=hex.spec.js.map
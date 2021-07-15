"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.fromHex = exports.toHex = void 0;
function toHex(data) {
    let out = "";
    for (const byte of data) {
        out += ("0" + byte.toString(16)).slice(-2);
    }
    return out;
}
exports.toHex = toHex;
function fromHex(hexstring) {
    if (hexstring.length % 2 !== 0) {
        throw new Error("hex string length must be a multiple of 2");
    }
    const listOfInts = [];
    for (let i = 0; i < hexstring.length; i += 2) {
        const hexByteAsString = hexstring.substr(i, 2);
        if (!hexByteAsString.match(/[0-9a-f]{2}/i)) {
            throw new Error("hex string contains invalid characters");
        }
        listOfInts.push(parseInt(hexByteAsString, 16));
    }
    return new Uint8Array(listOfInts);
}
exports.fromHex = fromHex;
//# sourceMappingURL=hex.js.map
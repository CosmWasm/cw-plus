"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const arrays_1 = require("./arrays");
describe("array", () => {
    describe("arrayContentEquals", () => {
        it("can compare number arrays", () => {
            expect(arrays_1.arrayContentEquals([1, 2, 3], [1, 2, 3])).toEqual(true);
            expect(arrays_1.arrayContentEquals([1, 2, 3], [1, 2, 3, 4])).toEqual(false);
            expect(arrays_1.arrayContentEquals([1, 2, 3], [3, 2, 1])).toEqual(false);
        });
        it("can compare string arrays", () => {
            expect(arrays_1.arrayContentEquals(["a", "b"], ["a", "b"])).toEqual(true);
            expect(arrays_1.arrayContentEquals(["a", "b"], ["a", "b", "c"])).toEqual(false);
            expect(arrays_1.arrayContentEquals(["a", "b"], ["b", "a"])).toEqual(false);
        });
        it("can compare bool arrays", () => {
            expect(arrays_1.arrayContentEquals([true, false], [true, false])).toEqual(true);
            expect(arrays_1.arrayContentEquals([true, false], [true, false, true])).toEqual(false);
            expect(arrays_1.arrayContentEquals([true, false], [false, true])).toEqual(false);
        });
        it("can compare different array types", () => {
            expect(arrays_1.arrayContentEquals([1, 2, 3], new Uint8Array([1, 2, 3]))).toEqual(true);
            expect(arrays_1.arrayContentEquals([1, 2, 3], new Uint8Array([3, 2, 1]))).toEqual(false);
        });
        it("works for empty arrays", () => {
            expect(arrays_1.arrayContentEquals([], [])).toEqual(true);
            expect(arrays_1.arrayContentEquals([], new Uint8Array([]))).toEqual(true);
        });
    });
    describe("arrayContentStartsWith", () => {
        it("can compare number arrays", () => {
            // same length
            expect(arrays_1.arrayContentStartsWith([], [])).toEqual(true); // Same behaviour as "".startsWith("")
            expect(arrays_1.arrayContentStartsWith([1, 2, 3], [1, 2, 3])).toEqual(true);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3], [1, 2, 8])).toEqual(false);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3], [0, 0, 0])).toEqual(false);
            // a shorter than b
            expect(arrays_1.arrayContentStartsWith([], [1, 2, 3])).toEqual(false);
            expect(arrays_1.arrayContentStartsWith([1], [1, 2, 3])).toEqual(false);
            expect(arrays_1.arrayContentStartsWith([1, 2], [1, 2, 3])).toEqual(false);
            // a longer than b
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [1, 2, 3, 4, 5])).toEqual(true);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [1, 2, 3, 4])).toEqual(true);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [1, 2, 3])).toEqual(true);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [1, 2])).toEqual(true);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [1])).toEqual(true);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [])).toEqual(true);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [1, 2, 3, 4, 0])).toEqual(false);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [1, 2, 3, 0])).toEqual(false);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [1, 2, 0])).toEqual(false);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [1, 0])).toEqual(false);
            expect(arrays_1.arrayContentStartsWith([1, 2, 3, 4, 5], [0])).toEqual(false);
        });
    });
});
//# sourceMappingURL=array.spec.js.map
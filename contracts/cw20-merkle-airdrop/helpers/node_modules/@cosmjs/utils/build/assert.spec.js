"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const assert_1 = require("./assert");
describe("assert", () => {
    describe("assertDefined", () => {
        it("passes for simple values", () => {
            {
                const value = 123;
                assert_1.assertDefined(value);
                expect(value).toEqual(123);
            }
            {
                const value = "abc";
                assert_1.assertDefined(value);
                expect(value).toEqual("abc");
            }
        });
        it("passes for falsy values", () => {
            {
                const value = 0;
                assert_1.assertDefined(value);
                expect(value).toEqual(0);
            }
            {
                const value = "";
                assert_1.assertDefined(value);
                expect(value).toEqual("");
            }
            {
                const value = null;
                assert_1.assertDefined(value);
                expect(value).toBeNull();
            }
        });
        it("throws for undefined values", () => {
            {
                const value = undefined;
                expect(() => assert_1.assertDefined(value)).toThrowError("value is undefined");
            }
            {
                let value;
                expect(() => assert_1.assertDefined(value)).toThrowError("value is undefined");
            }
        });
        it("throws with custom message", () => {
            const value = undefined;
            expect(() => assert_1.assertDefined(value, "Bug in the data source")).toThrowError("Bug in the data source");
        });
    });
    describe("assertDefinedAndNotNull", () => {
        it("passes for simple values", () => {
            {
                const value = 123;
                assert_1.assertDefinedAndNotNull(value);
                expect(value).toEqual(123);
            }
            {
                const value = "abc";
                assert_1.assertDefinedAndNotNull(value);
                expect(value).toEqual("abc");
            }
        });
        it("passes for falsy values", () => {
            {
                const value = 0;
                assert_1.assertDefinedAndNotNull(value);
                expect(value).toEqual(0);
            }
            {
                const value = "";
                assert_1.assertDefinedAndNotNull(value);
                expect(value).toEqual("");
            }
        });
        it("throws for undefined values", () => {
            {
                const value = undefined;
                expect(() => assert_1.assertDefinedAndNotNull(value)).toThrowError("value is undefined or null");
            }
            {
                let value;
                expect(() => assert_1.assertDefinedAndNotNull(value)).toThrowError("value is undefined or null");
            }
        });
        it("throws for null values", () => {
            const value = null;
            expect(() => assert_1.assertDefinedAndNotNull(value)).toThrowError("value is undefined or null");
        });
        it("throws with custom message", () => {
            const value = undefined;
            expect(() => assert_1.assertDefinedAndNotNull(value, "Bug in the data source")).toThrowError("Bug in the data source");
        });
    });
});
//# sourceMappingURL=assert.spec.js.map
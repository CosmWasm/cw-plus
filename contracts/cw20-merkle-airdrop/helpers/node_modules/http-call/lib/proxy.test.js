"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const proxy_1 = require("./proxy");
beforeEach(() => {
    proxy_1.default.env = {};
});
test('returns nothing', () => {
    expect(proxy_1.default.agent(true)).toBeUndefined();
});
describe('with proxies', () => {
    beforeEach(() => {
        proxy_1.default.env.HTTP_PROXY = 'http://user:pass@foo.com';
        proxy_1.default.env.HTTPS_PROXY = 'https://user:pass@bar.com';
    });
    test('has http properties', () => {
        expect(proxy_1.default.agent(false)).toMatchObject({
            options: {
                proxy: {
                    host: 'foo.com',
                    port: '8080',
                    proxyAuth: 'user:pass',
                },
            },
            proxyOptions: {
                host: 'foo.com',
                port: '8080',
                proxyAuth: 'user:pass',
            },
        });
    });
    test('has https properties', () => {
        expect(proxy_1.default.agent(true)).toMatchObject({
            defaultPort: 443,
            options: {
                proxy: {
                    host: 'bar.com',
                    port: '8080',
                    proxyAuth: 'user:pass',
                },
            },
            proxyOptions: {
                host: 'bar.com',
                port: '8080',
                proxyAuth: 'user:pass',
            },
        });
    });
});
describe('with http proxy only', () => {
    beforeEach(() => {
        proxy_1.default.env.HTTP_PROXY = 'http://user:pass@foo.com';
    });
    test('has agent', () => {
        expect(proxy_1.default.agent(true)).toMatchObject({
            defaultPort: 443,
            options: {
                proxy: {
                    host: 'foo.com',
                    port: '8080',
                    proxyAuth: 'user:pass',
                },
            },
            proxyOptions: {
                host: 'foo.com',
                port: '8080',
                proxyAuth: 'user:pass',
            },
        });
    });
});
describe('with no_proxy', () => {
    beforeEach(() => {
        proxy_1.default.env.HTTP_PROXY = 'http://user:pass@foo.com';
        proxy_1.default.env.NO_PROXY = 'some.com,test-domain.com';
    });
    test('is an exact match of no_proxy', () => {
        expect(proxy_1.default.agent(false, 'test-domain.com')).toBeUndefined();
    });
    test('is a subdomain of no_proxy', () => {
        expect(proxy_1.default.agent(false, 'something.prod.test-domain.com')).toBeUndefined();
    });
    test('should be proxied', () => {
        expect(proxy_1.default.agent(false, 'proxied-domain.com')).toMatchObject({
            options: {
                proxy: {
                    host: 'foo.com',
                    port: '8080',
                    proxyAuth: 'user:pass',
                },
            },
            proxyOptions: {
                host: 'foo.com',
                port: '8080',
                proxyAuth: 'user:pass',
            },
        });
    });
});
describe('proxy dodging', () => {
    test('not set should proxy', () => {
        proxy_1.default.env.NO_PROXY = '';
        expect(proxy_1.default.shouldDodgeProxy('test-domain.com')).toBe(false);
        expect(proxy_1.default.shouldDodgeProxy('other-domain.com')).toBe(false);
    });
    test('wildcard proxies any', () => {
        proxy_1.default.env.NO_PROXY = '*';
        expect(proxy_1.default.shouldDodgeProxy('test-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('anything.other-domain.com')).toBe(true);
    });
    test('exact domain should also match subdomains', () => {
        proxy_1.default.env.NO_PROXY = 'test-domain.com';
        expect(proxy_1.default.shouldDodgeProxy('test-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('anything.test-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('other-domain.com')).toBe(false);
        expect(proxy_1.default.shouldDodgeProxy('anything.other-domain.com')).toBe(false);
    });
    test('any sub domain should include the domain itself', () => {
        proxy_1.default.env.NO_PROXY = '.test-domain.com';
        expect(proxy_1.default.shouldDodgeProxy('test-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('anything.test-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('other-domain.com')).toBe(false);
        expect(proxy_1.default.shouldDodgeProxy('anything.other-domain.com')).toBe(false);
    });
    test('multiple domains', () => {
        proxy_1.default.env.NO_PROXY = '.test-domain.com, .other-domain.com';
        expect(proxy_1.default.shouldDodgeProxy('test-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('anything.test-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('other-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('anything.other-domain.com')).toBe(true);
    });
    test('match any subdomains', () => {
        proxy_1.default.env.NO_PROXY = '.test-domain.com, other-domain.com';
        expect(proxy_1.default.shouldDodgeProxy('test-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('something.something-else.anything.test-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('other-domain.com')).toBe(true);
        expect(proxy_1.default.shouldDodgeProxy('something.anything.other-domain.com')).toBe(true);
    });
});

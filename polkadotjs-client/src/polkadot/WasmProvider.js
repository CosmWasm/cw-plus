// Copyright 2018-2020 @paritytech/substrate-light-ui authors & contributors
// This software may be modified and distributed under the terms
// of the Apache-2.0 license. See the LICENSE file for details.
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, privateMap, value) {
    if (!privateMap.has(receiver)) {
        throw new TypeError("attempted to set private field on non-instance");
    }
    privateMap.set(receiver, value);
    return value;
};
var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, privateMap) {
    if (!privateMap.has(receiver)) {
        throw new TypeError("attempted to get private field on non-instance");
    }
    return privateMap.get(receiver);
};
var _coder, _eventemitter, _isConnected, _rpcClient;
import Coder from '@polkadot/rpc-provider/coder';
import { assert } from '@polkadot/util';
import EventEmitter from 'eventemitter3';
console.timeLog('wasm-provider');
export class WasmProvider {
    constructor(light) {
        _coder.set(this, void 0);
        _eventemitter.set(this, void 0);
        _isConnected.set(this, false);
        _rpcClient.set(this, undefined);
        __classPrivateFieldSet(this, _eventemitter, new EventEmitter());
        __classPrivateFieldSet(this, _coder, new Coder());
        this.light = light;
        this.connect();
    }
    /**
     * @summary `true` when this provider supports subscriptions
     */
    get hasSubscriptions() {
        return true;
    }
    /**
     * @description Returns a clone of the object
     */
    clone() {
        throw new Error('clone() is unimplemented yet.');
    }
    connect() {
        return this.light
            .startClient()
            .then((rpcClient) => {
            __classPrivateFieldSet(this, _rpcClient, rpcClient);
            __classPrivateFieldSet(this, _isConnected, true);
            this.emit('connected');
        })
            .catch((error) => {
            console.error(error);
        });
    }
    /**
     * @description Manually disconnect from the connection.
     */
    // eslint-disable-next-line @typescript-eslint/require-await
    disconnect() {
        return __awaiter(this, void 0, void 0, function* () {
            console.log('Destroying WASM light client');
            try {
                if (__classPrivateFieldGet(this, _rpcClient)) {
                    return __classPrivateFieldGet(this, _rpcClient).free();
                }
            }
            catch (error) {
                console.error(error);
                throw error;
            }
        });
    }
    /**
     * @summary Whether the node is connected or not.
     * @return {boolean} true if connected
     */
    get isConnected() {
        return __classPrivateFieldGet(this, _isConnected);
    }
    /**
     * @summary Listens on events after having subscribed using the [[subscribe]] function.
     * @param type - Event
     * @param sub - Callback
     */
    on(type, sub) {
        __classPrivateFieldGet(this, _eventemitter).on(type, sub);
        return () => {
            __classPrivateFieldGet(this, _eventemitter).removeListener(type, sub);
        };
    }
    /**
     * @summary Send JSON data using WebSockets to the wasm node.
     * @param method The RPC methods to execute
     * @param params Encoded paramaters as appliucable for the method
     * @param subscription Subscription details (internally used)
     */
    send(method, 
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    params, subscription
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    ) {
        if (subscription) {
            const json = __classPrivateFieldGet(this, _coder).encodeJson(method, params);
            console.log(() => ['calling', method, json]);
            assert(__classPrivateFieldGet(this, _rpcClient), 'Please call `send` after WasmProvider is ready');
            __classPrivateFieldGet(this, _rpcClient).rpcSubscribe(json, (response) => {
                try {
                    const result = __classPrivateFieldGet(this, _coder).decodeResponse(JSON.parse(response));
                    subscription.callback(null, result);
                }
                catch (error) {
                    subscription.callback(error, null);
                }
            });
            return Promise.resolve(0); // TODO subscriptionId
        }
        return new Promise((resolve, reject) => {
            try {
                const json = __classPrivateFieldGet(this, _coder).encodeJson(method, params);
                console.log(() => ['calling', method, json]);
                assert(__classPrivateFieldGet(this, _rpcClient), 'Please call `send` after WasmProvider is ready');
                __classPrivateFieldGet(this, _rpcClient).rpcSend(json).then((response) => {
                    try {
                        const result = __classPrivateFieldGet(this, _coder).decodeResponse(JSON.parse(response));
                        resolve(result);
                    }
                    catch (error) {
                        reject(error);
                    }
                });
            }
            catch (error) {
                reject(error);
            }
        });
    }
    /**
     * @name subscribe
     * @summary Allows subscribing to a specific event.
     * @param  {string}                     type     Subscription type
     * @param  {string}                     method   Subscription method
     * @param  {any[]}                 params   Parameters
     * @param  {ProviderInterfaceCallback} callback Callback
     * @return {Promise<number>}                     Promise resolving to the dd of the subscription you can use with [[unsubscribe]].
     *
     * @example
     * <BR>
     *
     * ```javascript
     * const provider = new WasmProvider(client);
     * const rpc = new Rpc(provider);
     *
     * rpc.state.subscribeStorage([[storage.balances.freeBalance, <Address>]], (_, values) => {
     *   console.log(values)
     * }).then((subscriptionId) => {
     *   console.log('balance changes subscription id: ', subscriptionId)
     * })
     * ```
     */
    subscribe(type, method, 
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    params, callback) {
        return __awaiter(this, void 0, void 0, function* () {
            const id = yield this.send(method, params, { callback, type });
            return id;
        });
    }
    /**
     * @summary Allows unsubscribing to subscriptions made with [[subscribe]].
     */
    unsubscribe(_type, method, id) {
        return __awaiter(this, void 0, void 0, function* () {
            const result = yield this.send(method, [id]);
            return result;
        });
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    emit(type, ...args) {
        __classPrivateFieldGet(this, _eventemitter).emit(type, ...args);
    }
}
_coder = new WeakMap(), _eventemitter = new WeakMap(), _isConnected = new WeakMap(), _rpcClient = new WeakMap();

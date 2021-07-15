const createKeccakHash = require('keccak')
const BN = require('bn.js')

function keccak256(a) {
  a = toBuffer(a)

  return createKeccakHash('keccak256').update(a).digest()
}

function toBuffer (v) {
  if (!Buffer.isBuffer(v)) {
    if (Array.isArray(v)) {
      v = Buffer.from(v)
    } else if (typeof v === 'string') {
      if (isHexString(v)) {
        v = Buffer.from(padToEven(stripHexPrefix(v)), 'hex')
      } else {
        v = Buffer.from(v)
      }
    } else if (typeof v === 'number') {
      v = intToBuffer(v)
    } else if (v === null || v === undefined) {
      v = Buffer.allocUnsafe(0)
    } else if (BN.isBN(v)) {
      v = v.toArrayLike(Buffer)
    } else if (v.toArray) {
      // converts a BN to a Buffer
      v = Buffer.from(v.toArray())
    } else {
      throw new Error('invalid type')
    }
  }
  return v
}

function isHexString (value, length) {
  if (typeof (value) !== 'string' || !value.match(/^0x[0-9A-Fa-f]*$/)) {
    return false
  }

  if (length && value.length !== 2 + 2 * length) { return false }

  return true
}

function padToEven (value) {
  var a = value; // eslint-disable-line

  if (typeof a !== 'string') {
    throw new Error(`while padding to even, value must be string, is currently ${typeof a}, while padToEven.`)
  }

  if (a.length % 2) {
    a = `0${a}`
  }

  return a
}

function stripHexPrefix (str) {
  if (typeof str !== 'string') {
    return str
  }

  return isHexPrefixed(str) ? str.slice(2) : str
}

function isHexPrefixed (str) {
  if (typeof str !== 'string') {
    throw new Error("value must be type 'string', is currently type " + (typeof str) + ', while checking isHexPrefixed.')
  }

  return str.slice(0, 2) === '0x'
}

function intToBuffer (i) {
  const hex = intToHex(i)

  return Buffer.from(padToEven(hex.slice(2)), 'hex')
}

function intToHex (i) {
  var hex = i.toString(16); // eslint-disable-line

  return `0x${hex}`
}

if (typeof window !== 'undefined') {
  window.keccak256 = keccak256
}

module.exports = keccak256

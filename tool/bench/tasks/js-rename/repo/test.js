import assert from 'node:assert/strict'
import { greet } from './greet.js'
assert.equal(greet(1), 'oi, ana')
console.log('ok')

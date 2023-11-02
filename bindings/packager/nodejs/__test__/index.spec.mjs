import test from 'ava'

import { logError } from '../index.js'

test('log error', (t) => {
  t.is(logError("unexpected argument"), undefined)
})

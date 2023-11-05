import test from 'ava'
import process from 'process'

import {
  bundleApp
} from '../build/index.js'

test('log error', async (t) => {
  process.env.CI = true
  process.chdir('../../../examples/electron')
  t.is(await bundleApp(), undefined)
})
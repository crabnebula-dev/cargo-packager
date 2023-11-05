import test from 'ava'
import process from 'process'
import { execSync } from "child_process"

import { bundleApp } from '../build/index.js'

test('log error', async (t) => {
  process.env.CI = true
  process.chdir('../../../examples/electron')
  execSync('yarn install')
  t.is(await bundleApp({}, { verbosity: 2 }), undefined)
})

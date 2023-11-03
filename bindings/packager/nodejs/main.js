const {
  cli: runCli,
  package: runPackager,
  logError
} = require('./index')
const {
  run: runPlugins
} = require('./plugins')
const merge = require('deepmerge')

async function package(config) {
  const conf = await runPlugins()

  let packagerConfig = config
  if (conf) {
    packagerConfig = merge(conf, config)
  }
  runPackager(JSON.stringify(packagerConfig))
}

async function cli(args, binName) {
  const config = await runPlugins()
  if (config) {
    args.push('--config')
    args.push(JSON.stringify(config))
  }
  runCli(args, binName)
}

module.exports = {
  cli,
  package,
  logError
}
const {
  cli,
  logError
} = require('./index')

module.exports.cli = (args, binName) => {
  return new Promise((resolve, reject) => {
    cli(args, binName, res => {
      if (res instanceof Error) {
        reject(res)
      } else {
        resolve(res)
      }
    })
  })
}

module.exports.logError = logError
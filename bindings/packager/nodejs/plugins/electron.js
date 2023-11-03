const fs = require('fs')
const path = require('path')
const os = require("os")
const {
  download: downloadElectron
} = require("@electron/get")
const extractZip = require('extract-zip')

function getPackageJsonPath() {
  let appDir = process.cwd()

  while (appDir.length && appDir[appDir.length - 1] !== path.sep) {
    const filepath = path.join(appDir, "package.json")
    if (fs.existsSync(filepath)) {
      return filepath
    }

    appDir = path.normalize(path.join(appDir, '..'))
  }

  return null
}

module.exports = async () => {
  const packageJsonPath = getPackageJsonPath()
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath))

  let electronPath
  try {
    electronPath = require.resolve("electron", {
      paths: [packageJsonPath]
    })
  } catch (e) {
    return
  }

  const electronPackageJson = JSON.parse(fs.readFileSync(path.resolve(path.dirname(electronPath), "package.json")))
  const zipPath = await downloadElectron(electronPackageJson.version)
  const zipDir = fs.mkdtempSync(os.tmpdir())
  await extractZip(zipPath, {
    dir: zipDir
  })

  const platformName = os.platform()
  let resources = []
  let frameworks = []
  let binaryPath
  switch (platformName) {
    case 'darwin':
      var standaloneElectronPath = path.join(zipDir, 'Electron.app')

      const resourcesPath = path.join(standaloneElectronPath, 'Contents/Resources')
      resources = fs.readdirSync(resourcesPath).map(p => path.join(resourcesPath, p))

      const frameworksPath = path.join(standaloneElectronPath, 'Contents/Frameworks')
      frameworks = fs.readdirSync(frameworksPath).map(p => path.join(frameworksPath, p))

      binaryPath = path.join(standaloneElectronPath, 'Contents/MacOS/Electron')
      break
    case 'win32':
      var standaloneElectronPath = path.join(zipDir, 'Electron.exe')
      binaryPath = standaloneElectronPath
      break
    default:
      var standaloneElectronPath = path.join(zipDir, 'Electron')
      binaryPath = standaloneElectronPath
  }

  return {
    name: packageJson.name,
    productName: packageJson.productName || packageJson.name,
    version: packageJson.version,
    resources,
    macos: {
      frameworks
    },
    binaries: [{
      path: binaryPath,
      main: true
    }]
  }
}
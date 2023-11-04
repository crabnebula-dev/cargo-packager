import type { Config, Resource } from "../../config";
import fs from "fs-extra";
import path from "path";
import os from "os";
import { download as downloadElectron } from "@electron/get";
import extractZip from "extract-zip";
import { Pruner, isModule, normalizePath } from "./prune";
import merge from "deepmerge";

function getPackageJsonPath(): string | null {
  let appDir = process.cwd();

  while (appDir.length && appDir[appDir.length - 1] !== path.sep) {
    const filepath = path.join(appDir, "package.json");
    if (fs.existsSync(filepath)) {
      return filepath;
    }

    appDir = path.normalize(path.join(appDir, ".."));
  }

  return null;
}

export default async function run(): Promise<Partial<Config> | null> {
  const packageJsonPath = getPackageJsonPath();

  if (packageJsonPath === null) {
    return null;
  }

  const packageJson = JSON.parse((await fs.readFile(packageJsonPath)).toString());

  let electronPath;
  try {
    electronPath = require.resolve("electron", {
      paths: [packageJsonPath],
    });
  } catch (e) {
    return null;
  }

  const userConfig = packageJson.packager || {};

  const electronPackageJson = JSON.parse(
    (await fs.readFile(path.resolve(path.dirname(electronPath), "package.json")))
      .toString()
  );

  // TODO: cache
  const zipPath = await downloadElectron(electronPackageJson.version);
  const zipDir = await fs.mkdtemp(path.join(os.tmpdir(), ".packager-electron"));
  await extractZip(zipPath, {
    dir: zipDir,
  });

  const platformName = os.platform();
  let resources: Resource[] = [];
  let frameworks: string[] = [];
  let debianFiles: {
    [k: string]: string;
  } | null = null;
  let binaryPath;

  const appPath = path.dirname(packageJsonPath);
  const appTempPath = await fs.mkdtemp(
    path.join(os.tmpdir(), packageJson.name || "app-temp")
  );

  const pruner = new Pruner(appPath, true);

  const outDir = userConfig.outDir ? path.resolve(userConfig.outDir) : null;
  const ignoredDirs = outDir && outDir !== process.cwd() ? [outDir] : [];

  const filterFunc = (_name: string): boolean => true;
  await fs.copy(appPath, appTempPath, {
    filter: async (file: string) => {
      const fullPath = path.resolve(file);

      if (ignoredDirs.includes(fullPath)) {
        return false;
      }

      let name = fullPath.split(appPath)[1];
      if (path.sep === "\\") {
        name = normalizePath(name);
      }

      if (name.startsWith("/node_modules/")) {
        if (await isModule(file)) {
          return await pruner.pruneModule(name);
        } else {
          return filterFunc(name);
        }
      }

      return filterFunc(name);
    },
  });

  switch (platformName) {
    case "darwin":
      var standaloneElectronPath = path.join(zipDir, "Electron.app");

      const resourcesPath = path.join(
        standaloneElectronPath,
        "Contents/Resources"
      );
      resources = resources.concat((await fs.readdir(resourcesPath))
        .filter((p) => p !== "default_app.asar")
        .map((p) => path.join(resourcesPath, p)));

      resources.push({
        src: appTempPath,
        target: "app",
      });

      const frameworksPath = path.join(
        standaloneElectronPath,
        "Contents/Frameworks"
      );
      frameworks = (await fs.readdir(frameworksPath))
        .map((p) => path.join(frameworksPath, p));

      binaryPath = path.join(standaloneElectronPath, "Contents/MacOS/Electron");
      break;
    case "win32":
      var standaloneElectronPath = path.join(zipDir, "Electron.exe");
      binaryPath = standaloneElectronPath;
      break;
    default:
      const binaryName = toKebabCase(userConfig.name || packageJson.productName || packageJson.name);

      // rename the electron binary
      await fs.rename(path.join(zipDir, 'electron'), path.join(zipDir, binaryName));

      const electronFiles = await fs.readdir(zipDir);

      const binTmpDir = await fs.mkdtemp(
        path.join(os.tmpdir(), `${packageJson.name || "app-temp"}-bin`)
      );
      binaryPath = path.join(binTmpDir, binaryName);
      await fs.writeFile(binaryPath, binaryScript(binaryName));
      await fs.chmod(binaryPath, 0o755);

      // make linuxdeploy happy
      process.env.LD_LIBRARY_PATH = process.env.LD_LIBRARY_PATH ? `${process.env.LD_LIBRARY_PATH}:${zipDir}` : zipDir
      // electron needs everything at the same level :)
      // resources only contains the default_app.asar so we ignore it
      debianFiles = electronFiles.filter(f => !['resources'].includes(f)).reduce((acc, file) => ({ ...acc, [path.join(zipDir, file)]: `usr/lib/${binaryName}/${file}` }), {});
      debianFiles[appTempPath] = `usr/lib/${binaryName}/resources/app`;

  }

  return merge({
    name: packageJson.name,
    productName: packageJson.productName || packageJson.name,
    version: packageJson.version,
    resources,
    macos: {
      frameworks,
    },
    deb: {
      files: debianFiles,
    },
    binaries: [
      {
        path: binaryPath,
        main: true,
      },
    ],
  }, userConfig);
}

const toKebabCase = (str: string) => str.split(/\.?(?=[A-Z])/).join('-').toLowerCase();

function binaryScript(binaryName: string): string {
  return `#!/usr/bin/env sh

full_path=$(realpath $0)
bin_dir_path=$(dirname $full_path)
usr_dir_path=$(dirname $bin_dir_path)
echo $usr_dir_path
$usr_dir_path/lib/${binaryName}/${binaryName}
`
}
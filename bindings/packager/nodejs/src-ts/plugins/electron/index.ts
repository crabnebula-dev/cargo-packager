import type { Config, Resource } from "../../config";
import fs from "fs-extra";
import path from "path";
import os from "os";
import { download as downloadElectron } from "@electron/get";
import extractZip from "extract-zip";
import { Pruner, isModule, normalizePath } from "./prune";

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

  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath).toString());

  let electronPath;
  try {
    electronPath = require.resolve("electron", {
      paths: [packageJsonPath],
    });
  } catch (e) {
    return null;
  }

  const electronPackageJson = JSON.parse(
    fs
      .readFileSync(path.resolve(path.dirname(electronPath), "package.json"))
      .toString()
  );

  // TODO: cache
  const zipPath = await downloadElectron(electronPackageJson.version);
  const zipDir = fs.mkdtempSync(path.join(os.tmpdir(), ".packager-electron"));
  await extractZip(zipPath, {
    dir: zipDir,
  });

  const platformName = os.platform();
  let resources: Resource[] = [];
  let frameworks: string[] = [];
  let binaryPath;
  switch (platformName) {
    case "darwin":
      var standaloneElectronPath = path.join(zipDir, "Electron.app");

      const resourcesPath = path.join(
        standaloneElectronPath,
        "Contents/Resources"
      );
      resources = fs
        .readdirSync(resourcesPath)
        .filter((p) => p !== "default_app.asar")
        .map((p) => path.join(resourcesPath, p));

      const appPath = path.dirname(packageJsonPath);
      const appTempPath = fs.mkdtempSync(
        path.join(os.tmpdir(), packageJson.name || "app-temp")
      );
      const pruner = new Pruner(appPath, true);
      const filterFunc = (_name: string): boolean => true;
      // TODO: we should also filter the output directory
      await fs.copy(appPath, appTempPath, {
        filter: async (file: string) => {
          const fullPath = path.resolve(file);
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

      resources.push({
        src: appTempPath,
        target: "app",
      });

      const frameworksPath = path.join(
        standaloneElectronPath,
        "Contents/Frameworks"
      );
      frameworks = fs
        .readdirSync(frameworksPath)
        .map((p) => path.join(frameworksPath, p));

      binaryPath = path.join(standaloneElectronPath, "Contents/MacOS/Electron");
      break;
    case "win32":
      var standaloneElectronPath = path.join(zipDir, "Electron.exe");
      binaryPath = standaloneElectronPath;
      break;
    default:
      var standaloneElectronPath = path.join(zipDir, "Electron");
      binaryPath = standaloneElectronPath;
  }

  return {
    name: packageJson.name,
    productName: packageJson.productName || packageJson.name,
    version: packageJson.version,
    resources,
    macos: {
      frameworks,
    },
    binaries: [
      {
        path: binaryPath,
        main: true,
      },
    ],
  };
}

import type { Config, Resource } from "../config";
import fs from "fs";
import path from "path";
import os from "os";
import { download as downloadElectron } from "@electron/get";
import extractZip from "extract-zip";

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
  const zipPath = await downloadElectron(electronPackageJson.version);
  const zipDir = fs.mkdtempSync(os.tmpdir());
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
        .map((p) => path.join(resourcesPath, p));
      resources.push({
        src: path.dirname(packageJsonPath),
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

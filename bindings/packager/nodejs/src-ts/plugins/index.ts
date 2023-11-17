import path from "path";
import fs from "fs-extra";
import type { Config } from "../config";
import electron from "./electron";
import merge from "deepmerge";

export interface PackageJson {
  name?: string;
  productName?: string;
  version?: string;
  packager: Partial<Config> | null | undefined;
}

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

  const packageJson = JSON.parse(
    (await fs.readFile(packageJsonPath)).toString()
  ) as PackageJson;

  let config = packageJson.packager || null;

  const electronConfig = await electron(
    path.dirname(packageJsonPath),
    packageJson
  );

  if (electronConfig) {
    config = config ? merge(electronConfig, config) : electronConfig;
  }

  if (config?.outDir) {
    await fs.ensureDir(config.outDir);
  }

  return config;
}

// from https://github.com/electron/electron-packager/blob/741f3c349e7f9e11e5ae14593a3efa79d312dc4d/src/prune.js

import { DestroyerOfModules, ModuleMap, DepType, Module } from "galactus";
import fs from "fs";
import path from "path";

const ELECTRON_MODULES = [
  "electron",
  "electron-nightly",
  "electron-prebuilt",
  "electron-prebuilt-compile",
];

export function normalizePath(path: string): string {
  return path.replace(/\\/g, "/");
}

class Pruner {
  baseDir: string;
  quiet: boolean;
  galactus: DestroyerOfModules;
  walkedTree: boolean;
  modules?: Set<string>;

  constructor(dir: string, quiet: boolean) {
    this.baseDir = normalizePath(dir);
    this.quiet = quiet;
    this.galactus = new DestroyerOfModules({
      rootDirectory: dir,
      shouldKeepModuleTest: (module, isDevDep) =>
        this.shouldKeepModule(module, isDevDep),
    });
    this.walkedTree = false;
  }

  setModules(moduleMap: ModuleMap) {
    const modulePaths = Array.from(moduleMap.keys()).map(
      (modulePath) => `/${normalizePath(modulePath)}`
    );
    this.modules = new Set(modulePaths);
    this.walkedTree = true;
  }

  async pruneModule(name: string) {
    if (this.walkedTree) {
      return this.isProductionModule(name);
    } else {
      const moduleMap = await this.galactus.collectKeptModules({
        relativePaths: true,
      });
      this.setModules(moduleMap);
      return this.isProductionModule(name);
    }
  }

  shouldKeepModule(module: Module, isDevDep: boolean) {
    if (isDevDep || module.depType === DepType.ROOT) {
      return false;
    }

    if (ELECTRON_MODULES.includes(module.name)) {
      if (!this.quiet)
        console.warn(
          `Found '${module.name}' but not as a devDependency, pruning anyway`
        );
      return false;
    }

    return true;
  }

  isProductionModule(name: string): boolean {
    return this.modules?.has(name) ?? false;
  }
}

function isNodeModuleFolder(pathToCheck: string) {
  return (
    path.basename(path.dirname(pathToCheck)) === "node_modules" ||
    (path.basename(path.dirname(pathToCheck)).startsWith("@") &&
      path.basename(path.resolve(pathToCheck, `..${path.sep}..`)) ===
        "node_modules")
  );
}

export async function isModule(pathToCheck: string) {
  return (
    (await fs.existsSync(path.join(pathToCheck, "package.json"))) &&
    isNodeModuleFolder(pathToCheck)
  );
}

export { Pruner };

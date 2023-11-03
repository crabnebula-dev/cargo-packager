import cargoPackager from "../index";
import runPlugins from "./plugins";
import merge from "deepmerge";
import type { Config } from "./config";

async function bundleApp(config: Config) {
  const conf = await runPlugins();

  let packagerConfig = config;
  if (conf) {
    packagerConfig = merge(conf, config);
  }
  cargoPackager.package(JSON.stringify(packagerConfig));
}

async function cli(args: string[], binName: string) {
  const config = await runPlugins();
  if (config) {
    args.push("--config");
    args.push(JSON.stringify(config));
  }
  cargoPackager.cli(args, binName);
}

function logError(error: string) {
  cargoPackager.logError(error);
}

export { cli, bundleApp, logError };

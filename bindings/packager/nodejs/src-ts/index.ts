import cargoPackager from "../index";
import runPlugins from "./plugins";
import merge from "deepmerge";
import type { Config } from "./config";

let tracingEnabled = false;

export interface Options {
  verbosity?: number;
}

export interface SigningConfig {
  /** The private key to use for signing. */
  privateKey: string;
  /**
   * The private key password.
   *
   * If `null`, user will be prompted to write a password.
   * You can skip the prompt by specifying an empty string.
   */
  password?: string;
}

async function packageApp(config: Config = {}, options?: Options) {
  const conf = await runPlugins();

  let packagerConfig = config;
  if (conf) {
    packagerConfig = merge(conf, config);
  }

  if (!tracingEnabled) {
    cargoPackager.initTracingSubscriber(options?.verbosity ?? 0);
    tracingEnabled = true;
  }

  cargoPackager.packageApp(JSON.stringify(packagerConfig));
}

async function packageAndSignApp(
  config: Config = {},
  signingConfig: SigningConfig,
  options?: Options
) {
  const conf = await runPlugins();

  let packagerConfig = config;
  if (conf) {
    packagerConfig = merge(conf, config);
  }

  if (!tracingEnabled) {
    cargoPackager.initTracingSubscriber(options?.verbosity ?? 0);
    tracingEnabled = true;
  }

  cargoPackager.packageAndSignApp(
    JSON.stringify(packagerConfig),
    JSON.stringify(signingConfig)
  );
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

export { cli, packageApp, packageAndSignApp, logError };

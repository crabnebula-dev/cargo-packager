import test from "ava";
import process from "process";
import { execSync } from "child_process";

import { packageApp } from "../build/index.js";

test("log error", async (t) => {
  process.env.CI = true;
  process.chdir("../../../examples/electron");
  execSync("yarn install");
  t.is(
    await packageApp(
      {
        formats: process.env.PACKAGER_FORMATS
          ? process.env.PACKAGER_FORMATS.split(",")
          : null,
      },
      { verbosity: 2 }
    ),
    undefined
  );
});

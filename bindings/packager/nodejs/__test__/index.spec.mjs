import test from "ava";
import process from "process";
import { execSync } from "child_process";
import fs from "fs/promises";
import os from "os";
import path from "path";
import { packageApp } from "../build/index.js";
import { fileURLToPath } from "url";

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
      { verbosity: 2 },
    ),
    undefined,
  );
});

test("preserves executable file permissions when extracting a ZIP", async (t) => {
  const { extractZip } = await import("../build/plugins/electron/extract.js");
  const tempDir = await fs.mkdtemp(
    path.join(os.tmpdir(), "extract-permissions-"),
  );
  const zipPath = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    "fixtures",
    "permissions-test.zip",
  );
  const outputDir = path.join(tempDir, "output");

  await fs.mkdir(outputDir);

  try {
    await extractZip(zipPath, outputDir);

    if (process.platform === "win32") {
      t.pass();
      return;
    }

    const stats = await fs.stat(
      path.join(outputDir, "permissions-input", "test.sh"),
    );

    t.is(stats.mode & 0o777, 0o755);
  } finally {
    await fs.rm(tempDir, { recursive: true, force: true });
  }
});

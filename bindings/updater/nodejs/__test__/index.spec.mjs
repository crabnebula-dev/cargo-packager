import test from "ava";
import { writeFile, stat, readFile, rename } from "fs/promises";
import { join, format, parse } from "path";
import { execa } from "execa";
import { fileURLToPath } from "url";
import { App } from "@tinyhttp/app";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const UPDATER_PRIVATE_KEY =
  "dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5VU1qSHBMT0E4R0JCVGZzbUMzb3ZXeGpGY1NSdm9OaUxaVTFuajd0T2ZKZ0FBQkFBQUFBQUFBQUFBQUlBQUFBQWlhRnNPUmxKWjBiWnJ6M29Cd0RwOUpqTW1yOFFQK3JTOGdKSi9CajlHZktHajI2ZnprbEM0VUl2MHhGdFdkZWpHc1BpTlJWK2hOTWo0UVZDemMvaFlYVUM4U2twRW9WV1JHenNzUkRKT2RXQ1FCeXlkYUwxelhacmtxOGZJOG1Nb1R6b0VEcWFLVUk9Cg==";

test("it works", async (t) => {
  const isWindows = process.platform === "win32";
  const isMacos = process.platform === "darwin";
  const appDir = join(__dirname, "app");
  const target = `${isWindows ? "windows" : isMacos ? "macos" : "linux"}-${
    process.arch === "x64" ? "x86_64" : "i686"
  }`;

  await execa("pnpm", ["build"], {
    cwd: join(__dirname, "..", "..", "..", "packager", "nodejs"),
  });

  const buildApp = async (version, updaterFormats) => {
    await writeFile(
      join(appDir, "dist", "ver.js"),
      `module.exports.version = "${version}";`,
    );

    try {
      await execa(
        "pnpm",
        [
          "packager",
          "--verbose",
          "-f",
          updaterFormats.join(","),
          "-c",
          `{"outDir":"./dist","beforePackagingCommand": "pnpm build", "identifier": "com.updater-app-nodejs.test", "productName": "PackagerAppUpdaterTestNodejs", "version": "${version}", "icons": ["32x32.png"], "binaries": [{"path": "updater-app-test", "main": true}]}`,
        ],
        {
          stdio: "inherit",
          cwd: appDir,
          env: {
            CARGO_PACKAGER_SIGN_PRIVATE_KEY: UPDATER_PRIVATE_KEY,
            CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD: "",
          },
        },
      );
    } catch (e) {
      console.error("failed to package app");
      console.error(e);
    }
  };

  // bundle app update
  await buildApp(
    "1.0.0",
    isWindows ? ["nsis", "wix"] : isMacos ? ["app"] : ["appimage"],
  );

  const packgePaths = (version) => {
    return isWindows
      ? [
          [
            "nsis",
            join(appDir, "dist", `updater-app-test_${version}_x64-setup.exe`),
          ],
          [
            "wix",
            join(appDir, "dist", `updater-app-test_${version}_x64_en-US.msi`),
          ],
        ]
      : isMacos
        ? [["app", join(appDir, "dist", "PackagerAppUpdaterTestNodejs.app")]]
        : [
            [
              "appimage",
              join(
                appDir,
                "dist",
                `updater-app-test_${version}_x86_64.AppImage`,
              ),
            ],
          ];
  };

  for (const [updaterFormat, outPackagePath] of packgePaths("1.0.0")) {
    const outUpdaterPath = (await stat(outPackagePath)).isDirectory()
      ? `${outPackagePath}.tar.gz`
      : outPackagePath;

    const signaturePath = format({ name: outUpdaterPath, ext: ".sig" });
    const signature = await readFile(signaturePath, { encoding: "utf8" });

    let updaterPath = outUpdaterPath;
    if (isMacos) {
      // we need to move it otherwise it'll be overwritten when we build the next app
      const info = parse(outUpdaterPath);
      updaterPath = format({
        dir: info.dir,
        base: `update-${info.base}`,
      });
      await rename(outUpdaterPath, updaterPath);
    }

    const server = new App()
      .get("/", (_, res) => {
        const platforms = {};
        platforms[target] = {
          signature,
          url: "http://localhost:3007/download",
          format: updaterFormat,
        };
        res.status(200).json({
          version: "1.0.0",
          date: new Date().toISOString(),
          platforms,
        });
      })
      .get("/download", (req, res) => {
        res.status(200).sendFile(updaterPath);
      })
      .listen(3007);

    // bundle initial app version
    await buildApp("0.1.0", [updaterFormat]);

    if (isWindows) {
      // install the app through the installer
      const isNsis = updaterFormat === "nsis";
      const installDir = join(appDir, "dist");
      const installerArg = `"${outPackagePath}"`;
      await execa("powershell.exe", [
        "-NoProfile",
        "-WindowStyle",
        "Hidden",
        "Start-Process",
        installerArg,
        "-ArgumentList",
        `${isNsis ? "/P" : "/passive"}, ${
          isNsis ? "/D" : "INSTALLDIR"
        }=${installDir}`,
      ]);
    }

    // wait 2secs to make sure the installer have released its lock on the binary
    await sleep(2000);

    const app = join(
      appDir,
      "dist",
      isWindows
        ? "updater-app-test.exe"
        : isMacos
          ? "PackagerAppUpdaterTestNodejs.app/Contents/MacOS/updater-app-test"
          : `updater-app-test_0.1.0_x86_64.AppImage`,
    );

    // wait until the update is finished and the new version has been installed
    // before starting another updater test, this is because we use the same starting binary
    // and we can't use it while the updater is installing it
    let counter = 0;
    while (true) {
      try {
        const { stdout, stderr } = await execa(app, [], {
          env: { UPDATER_FORMAT: updaterFormat },
        });
        const version = stdout.split("\n")[0];
        if (version === "1.0.0") {
          t.is(version, "1.0.0");
          console.log(`app is updated, new version: ${version}`);
          break;
        }

        console.log(`unexpected output: ${stdout}`);
        console.log(`stderr: ${stderr}`);
      } catch (e) {
        console.error("failed to check if app was updated");
        console.error(e);
      }

      counter += 1;
      if (counter == 10) {
        console.error(
          "updater test timedout and couldn't verify the update has happened",
        );
        break;
      }

      await sleep(2000);
    }

    server.close();
  }
});

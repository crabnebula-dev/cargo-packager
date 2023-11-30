import test from "ava";
import * as fs from "fs/promises";
import { existsSync } from "fs";
import * as path from "path";
import { execa } from "execa";
import { fileURLToPath } from "url";
import { App } from "@tinyhttp/app";
import { packageAndSignApp } from "@crabnebula/packager";

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const __dirname = fileURLToPath(new URL(".", import.meta.url));
const isWin = process.platform === "win32";
const isMac = process.platform === "darwin";

test("it updates correctly", async (t) => {
  const UPDATER_PRIVATE_KEY = await fs.readFile(
    path.join(__dirname, "../../../../crates/updater/tests/dummy.key"),
    { encoding: "utf8" },
  );

  process.chdir(path.join(__dirname, "app"));
  await execa("yarn", ["install"]);

  const buildApp = async (version, updaterFormats) => {
    const content = await fs.readFile("main.js", { encoding: "utf8" });
    await fs.writeFile("main.js", content.replace("{{version}}", version));

    try {
      await packageAndSignApp(
        {
          formats: updaterFormats,
          version,
        },
        {
          privateKey: UPDATER_PRIVATE_KEY,
          password: "",
        },
        { verbosity: 2 },
      );
    } catch (e) {
      console.error("failed to package app");
      console.error(e);
    } finally {
      const content = await fs.readFile("main.js", { encoding: "utf8" });
      await fs.writeFile("main.js", content.replace(version, "{{version}}"));
    }
  };

  // bundle app update
  const formats = isWin ? ["nsis", "wix"] : isMac ? ["app"] : ["appimage"];
  await buildApp("1.0.0", formats);

  const gneratedPackages = isWin
    ? [
        ["nsis", path.join("dist", `ElectronApp_1.0.0_x64-setup.exe`)],
        ["wix", path.join("dist", `ElectronApp_1.0.0_x64_en-US.msi`)],
      ]
    : isMac
      ? [["app", path.join("dist", "ElectronApp.app.tar.gz")]]
      : [["appimage", path.join("dist", `electron-app_1.0.0_x86_64.AppImage`)]];

  for (let [format, updatePackagePath] of gneratedPackages) {
    const signaturePath = path.format({ name: updatePackagePath, ext: ".sig" });
    const signature = await fs.readFile(signaturePath, { encoding: "utf8" });

    // on macOS, gnerated bundle doesn't have the version in its name
    // so we need to move it otherwise it'll be overwritten when we build the next app
    if (isMac) {
      const info = path.parse(updatePackagePath);
      const newPath = path.format({
        dir: info.dir,
        base: `update-1.0.0-${info.base}`,
      });
      await fs.rename(updatePackagePath, newPath);
      updatePackagePath = newPath;
    }

    // start the updater server
    const server = new App()
      .get("/", (_, res) => {
        const platforms = {};
        const target = `${isWin ? "windows" : isMac ? "macos" : "linux"}-${
          process.arch === "x64" ? "x86_64" : "i686"
        }`;
        platforms[target] = {
          signature,
          url: "http://localhost:3007/download",
          format,
        };
        res.status(200).json({
          version: "1.0.0",
          date: new Date().toISOString(),
          platforms,
        });
      })
      .get("/download", (_req, res) => {
        res
          .status(200)
          .sendFile(path.join(__dirname, "app", updatePackagePath));
      })
      .listen(3007);

    // bundle initial app version
    await buildApp("0.1.0", [format]);

    // install the inital app on Windows to `installdir`
    if (isWin) {
      const installDir = path.join(__dirname, "app", "dist", "installdir");
      if (existsSync(installDir)) await fs.rm(installDir, { recursive: true });
      await fs.mkdir(installDir);

      const isNsis = format === "nsis";

      const installerArg = `"${path.join(
        "dist",
        isNsis
          ? `ElectronApp_0.1.0_x64-setup.exe`
          : `ElectronApp_0.1.0_x64_en-US.msi`,
      )}"`;

      await execa("powershell.exe", [
        "-NoProfile",
        "-WindowStyle",
        "Hidden",
        "Start-Process",
        installerArg,
        "-Wait",
        "-ArgumentList",
        `${isNsis ? "/P" : "/passive"}, ${
          isNsis ? "/D" : "INSTALLDIR"
        }=${installDir}`,
      ]);
    }

    const app = path.join(
      "dist",
      isWin
        ? "installdir/ElectronApp.exe"
        : isMac
          ? "ElectronApp.app/Contents/MacOS/ElectronApp"
          : `electron-app_0.1.0_x86_64.AppImage`,
    );

    // save the current creation time
    const stats = await fs.stat(app);
    const ctime1 = stats.birthtime;

    // run initial app
    try {
      await execa(app, {
        stdio: "inherit",
        // This is read by the updater app test
        env: { UPDATER_FORMAT: format },
      });
    } catch (e) {
      console.error(`failed to start initial app: ${e}`);
    }

    // the test app is electron which is huge in size
    // and the installation takes a who;e
    // so wait 30 secs to make sure the installer has finished
    await sleep(30000);

    // wait until the update is finished and the new version has been installed
    // before starting another updater test, this is because we use the same starting binary
    // and we can't use it while the updater is installing it
    let counter = 0;
    while (true) {
      // check if the main binary creation time has changed since `ctime1`
      const stats = await fs.stat(app);
      if (ctime1 !== stats.birthtime) {
        try {
          const { stdout, stderr } = await execa(app);

          const lines = stdout.split(isWin ? "\r\n" : "\n");
          const version = lines.filter((l) => l)[0];

          if (version === "1.0.0") {
            console.log(`app is updated, new version: ${version}`);
            break;
          }

          console.log(`unexpected output (stdout): ${stdout}`);
          console.log(`stderr: ${stderr}`);
        } catch (e) {
          console.error(`failed to check if app was updated: ${e}`);
        }
      }

      counter += 1;
      if (counter == 10) {
        console.error(
          "updater test timedout and couldn't verify the update has happened",
        );
        break;
      }

      await sleep(5000);
    }

    server.close();
  }

  t.pass("Test successful");
});

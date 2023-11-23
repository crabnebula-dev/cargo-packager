const { checkUpdate } = require("../../index.js");
const { version } = require("./dist/ver.js");
const { join } = require("path");

console.log(version);

const updaterFormat = process.env["UPDATER_FORMAT"];
const appimg = process.env["APPIMAGE"];
const isLinux = process.platfrom !== "win32" && process.platfrom !== "darwin";

checkUpdate(version, {
  pubkey:
    "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQ2Njc0OTE5Mzk2Q0ExODkKUldTSm9XdzVHVWxuUmtJdjB4RnRXZGVqR3NQaU5SVitoTk1qNFFWQ3pjL2hZWFVDOFNrcEVvVlcK",
  endpoints: ["http://localhost:3007"],
  executablePath: isLinux && appimg ? appimg : undefined,
  windows: {
    installerArgs:
      // /D sets the default installation directory ($INSTDIR),
      // overriding InstallDir and InstallDirRegKey.
      // It must be the last parameter used in the command line and must not contain any quotes, even if the path contains spaces.
      // Only absolute paths are supported.
      // NOTE: we only need this because this is an integration test and we don't want to install the app in the programs folder
      updaterFormat === "nsis"
        ? [`/D=${join(process.execPath, "..")}`]
        : undefined,
  },
})
  .then((u) => {
    if (u) {
      u.downloadAndInstall()
        .then((_) => {
          process.exit(0);
        })
        .catch((e) => {
          console.error(e);
          process.exit(1);
        });
    } else {
      process.exit(0);
    }
  })
  .catch((e) => {
    console.error(e);
    process.exit(1);
  });

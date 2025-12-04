# cargo-packager-updater

Updater for apps that was packaged by [`cargo-packager`](https://docs.rs/cargo-packager).

## Checking for an update

you can check for an update using [`check_update`](https://docs.rs/cargo-packager-updater/latest/cargo_packager_updater/fn.check_update.html) function or construct a new [`Updater`](https://docs.rs/cargo-packager-updater/latest/cargo_packager_updater/struct.Updater.html)
using [`UpdaterBuilder`](https://docs.rs/cargo-packager-updater/latest/cargo_packager_updater/struct.UpdaterBuilder.html), both methods require the current version of the app and
a [`Config`](https://docs.rs/cargo-packager-updater/latest/cargo_packager_updater/struct.Config.html) that specifies the endpoints to request updates from and the public key of the update signature.

```rs
use cargo_packager_updater::{check_update, Config};

let config = Config {
  endpoints: vec!["http://myserver.com/updates".parse().unwrap()],
  pubkey: "<pubkey here>".into(),
  ..Default::default()
};
if let Some(update) = check_update("0.1.0".parse().unwrap(), config).expect("failed while checking for update") {
  update.download_and_install().expect("failed to download and install update");
} else {
  // there is no updates
}

```

## Endpoints

Each endpoint optionally could have `{{arch}}`, `{{target}}` or `{{current_version}}`
which will be detected and replaced with the appropriate value before making a request to the endpoint.

- `{{current_version}}`: The version of the app that is requesting the update.
- `{{target}}`: The operating system name (one of `linux`, `windows` or `macos`).
- `{{arch}}`: The architecture of the machine (one of `x86_64`, `i686`, `aarch64` or `armv7`).

for example:

```
 "https://releases.myapp.com/{{target}}/{{arch}}/{{current_version}}"
```

will turn into

```
 "https://releases.myapp.com/windows/x86_64/0.1.0"
```

if you need more data, you can set additional request headers [`UpdaterBuilder::header`](https://docs.rs/cargo-packager-updater/latest/cargo_packager_updater/struct.UpdaterBuilder.html#method.header) to your liking.

## Endpoint Response

The updater expects the endpoint to respond with 2 possible reponses:

1.  [`204 No Content`](https://datatracker.ietf.org/doc/html/rfc2616#section-10.2.5) in case there is no updates available.
2.  [`200 OK`](https://datatracker.ietf.org/doc/html/rfc2616#section-10.2.1) and a JSON response that could be either a JSON representing all available platform updates
    or if using endpoints variables (see above) or a header to attach the current updater target,
    then it can just return information for the requested target.

The JSON response is expected to have these fields set:

- `version`: must be a valid semver, with or without a leading `v``, meaning that both `1.0.0`and`v1.0.0`are valid.
- `url`or`platforms.[target].url`: must be a valid url to the update bundle.
- `signature`or`platforms.[target].signature`: must be the content of the generated `.sig`file. The signature may change each time you run build your app so make sure to always update it.
- `format`or`platforms.[target].format`: must be one of `app`, `appimage`, `nsis`or`wix`.

> [!NOTE]
> if using `platforms` object, each key is in the `OS-ARCH` format, where `OS` is one of `linux`, `macos` or `windows`, and `ARCH` is one of `x86_64`, `aarch64`, `i686` or `armv7`, see the example below.

It can also contain these optional fields:

- `notes`: Here you can add notes about the update, like release notes.
- `pub_date`: must be formatted according to [RFC 3339](https://datatracker.ietf.org/doc/html/rfc3339#section-5.8) if present.

Here is an example of the two expected JSON formats:

- **JSON for all platforms**

  ```json
  {
    "version": "v1.0.0",
    "notes": "Test version",
    "pub_date": "2020-06-22T19:25:57Z",
    "platforms": {
      "macos-x86_64": {
        "signature": "Content of app.tar.gz.sig",
        "url": "https://github.com/username/reponame/releases/download/v1.0.0/app-x86_64.app.tar.gz",
        "format": "app"
      },
      "macos-aarch64": {
        "signature": "Content of app.tar.gz.sig",
        "url": "https://github.com/username/reponame/releases/download/v1.0.0/app-aarch64.app.tar.gz",
        "format": "app"
      },
      "linux-x86_64": {
        "signature": "Content of app.AppImage.sig",
        "url": "https://github.com/username/reponame/releases/download/v1.0.0/app-amd64.AppImage.tar.gz",
        "format": "appimage"
      },
      "windows-x86_64": {
        "signature": "Content of app-setup.exe.sig or app.msi.sig, depending on the chosen format",
        "url": "https://github.com/username/reponame/releases/download/v1.0.0/app-x64-setup.nsis.zip",
        "format": "nsis or wix depending on the chosen format"
      }
    }
  }
  ```

- **JSON for one platform**

  ```json
  {
    "version": "0.2.0",
    "pub_date": "2020-09-18T12:29:53+01:00",
    "url": "https://mycompany.example.com/myapp/releases/myrelease.tar.gz",
    "signature": "Content of the relevant .sig file",
    "format": "app or nsis or wix or appimage depending on the release target and the chosen format",
    "notes": "These are some release notes"
  }
  ```

## Update install mode on Windows

You can specify which install mode to use on Windows using [`WindowsConfig::install_mode`](https://docs.rs/cargo-packager-updater/latest/cargo_packager_updater/struct.WindowsConfig.html#structfield.install_mode) which can be on of:

- `"Passive"`: There will be a small window with a progress bar. The update will be installed without requiring any user interaction. Generally recommended and the default mode.
- `"BasicUi"`: There will be a basic user interface shown which requires user interaction to finish the installation.
- `"Quiet"`: There will be no progress feedback to the user. With this mode the installer cannot request admin privileges by itself so it only works in user-wide installations or when your app itself already runs with admin privileges. Generally not recommended.

## Licenses

MIT or MIT/Apache 2.0 where applicable.

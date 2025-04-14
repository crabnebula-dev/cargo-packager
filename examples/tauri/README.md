## Tauri example

1. install `tauri-cli` first

   ```sh
   cargo install tauri-cli --version "2.0.0-rc.10" --locked
   ```

2. Change `UPDATER_ENDPOINT` value in `src/main.rs` to point to your updater server or static update file.
3. package the app

   ```sh
   cargo r -p cargo-packager -- -p tauri-example  --release --private-key dummy.key --password ""
   ```

4. increase the version in `Cargo.toml`
5. do step 3 again
6. upload the resulting package from step 5 to your endpoint
7. run the app generated from step 3

## Wails example

1. install The Go programming language: https://go.dev/dl/
2. install `wails` CLI first

   ```sh
   go install github.com/wailsapp/wails/v2/cmd/wails@latest
   ```

3. package the app

   ```sh
   cargo r -p cargo-packager -- -p wails-example --release
   ```

#!/usr/bin/env -S pkgx +npm +go +gum +jq zsh
# ^^ curl https://pkgx.sh | sh
# ^^ pkgx makes all those tools (including bash!) available to the script
#    no packages are installed; your system remains pristine

go install github.com/wailsapp/wails/v2/cmd/wails@latest

# works on mac
export PATH="$HOME/go/bin:$PATH"

if [ -d wails_example ]; then
  cd wails_example
elif [ ! -d .git ] && gum confirm 'Create new wails app?'; then
  wails init -n wails_example -t vanilla
  cd wails_example
fi

# probably not resilient if wails changes
wails build | grep "Built" | cut -d " " -f 2 | read buildpath

echo "Your binary is available at ${buildpath}"

# this is broken and I am lost like a donkey at Karneval
cargo r -p cargo-packager -- -p wails-example --release --config binaries=`[{ "filename": ${buildpath}, "main": true }]`

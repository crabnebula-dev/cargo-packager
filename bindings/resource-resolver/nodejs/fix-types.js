// Due to a NAPI-rs bug? still unconfirmed
// index.d.ts will contain a duplicate definition of `PackageFromat` enum
// and we only need the second definition.
// This script sole purpose is to remove the extra definition.

const { readFileSync, writeFileSync } = require("fs");
const { join } = require("path");

const typesPath = join(__dirname, "index.d.ts");
const types = readFileSync(typesPath, "utf8");

let out = "";
let inRemoval = false;
for (const line of types.split("\n")) {
  if (inRemoval) {
    if (line === "}") inRemoval = false;
    continue;
  }

  const startOfRemoval = line.startsWith(
    "/** Types of supported packages by [`cargo-packager`](https://docs.rs/cargo-packager). */",
  );
  if (startOfRemoval) {
    inRemoval = true;
    continue;
  }

  out += line + "\n";
}

writeFileSync(typesPath, out);

const problematicCode = `const { PackageFormat, PackageFormat, resourcesDir } = nativeBinding

module.exports.PackageFormat = PackageFormat
module.exports.PackageFormat = PackageFormat
module.exports.resourcesDir = resourcesDir`;
const correctCode = `const { PackageFormat, resourcesDir } = nativeBinding

module.exports.PackageFormat = PackageFormat
module.exports.resourcesDir = resourcesDir`;

const indexPath = join(__dirname, "index.js");
const indexContent = readFileSync(indexPath, "utf8");

writeFileSync(indexPath, indexContent.replace(problematicCode, correctCode));

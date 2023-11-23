const { compileFromFile } = require("json-schema-to-typescript");
const fs = require("fs");
const path = require("path");

// compile from file
compileFromFile(
  path.join(__dirname, "../../../crates/packager/schema.json"),
).then((ts) => fs.writeFileSync("src-ts/config.d.ts", ts));

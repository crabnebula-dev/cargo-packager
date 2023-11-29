const { compileFromFile } = require("json-schema-to-typescript");
const fs = require("fs");
const path = require("path");

// compile from file
compileFromFile(
  path.join(__dirname, "../../../crates/packager/schema.json"),
).then((ts) => {
  for (const dir of ["src-ts", "build"]) {
    try {
      fs.mkdirSync(dir);
    } catch (_) {}
    fs.writeFileSync(path.join(dir, "config.d.ts"), ts);
  }
});

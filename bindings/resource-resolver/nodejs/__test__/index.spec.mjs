import test from "ava";

import { resourcesDir, PackageFormat } from "../index.js";

test("resolve resource directory", async (t) => {
  const dir = resourcesDir(PackageFormat.Nsis);
  t.is(typeof dir, "string");
});

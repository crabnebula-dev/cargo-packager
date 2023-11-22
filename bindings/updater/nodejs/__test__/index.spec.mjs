import test from "ava";

import { checkUpdate } from "../index.js";

test("it checks for update", async (t) => {
  await checkUpdate("0.1.0", {
    pubkey:
      "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQ2Njc0OTE5Mzk2Q0ExODkKUldTSm9XdzVHVWxuUmtJdjB4RnRXZGVqR3NQaU5SVitoTk1qNFFWQ3pjL2hZWFVDOFNrcEVvVlcK",
    endpoints: ["http://localhost:2342"],
  });
});

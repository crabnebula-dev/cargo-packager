import type { Config } from "..";
import electron from "./electron";

export default async function run(): Promise<Partial<Config> | null> {
  return await electron();
}

import type { Config } from "../config";
import electron from "./electron";

export default async function run(): Promise<Partial<Config> | null> {
  return await electron();
}

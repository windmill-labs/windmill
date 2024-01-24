import { yamlParse } from "./deps.ts";

export interface WmillConfiguration {
  includes?: string[];
  excludes?: string[];
}
export async function readConfigFile(): Promise<WmillConfiguration> {
  try {
    const conf = yamlParse(
      await Deno.readTextFile("wmill.yaml")
    ) as WmillConfiguration;

    return typeof conf == "object" ? conf : ({} as WmillConfiguration);
  } catch (e) {
    return {};
  }
}

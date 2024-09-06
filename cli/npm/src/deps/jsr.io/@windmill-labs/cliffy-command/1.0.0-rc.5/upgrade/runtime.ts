import type { Logger } from "./logger.js";
import type { Provider, ProviderUpgradeOptions } from "./provider.js";

/** Options for upgrading a package for a specific runtime. */
export interface RuntimeUpgradeOptions extends ProviderUpgradeOptions {
  provider: Provider;
  logger?: Logger;
}

/** Runtime handler. */
export abstract class Runtime {
  abstract upgrade(_options: RuntimeUpgradeOptions): Promise<void>;
}

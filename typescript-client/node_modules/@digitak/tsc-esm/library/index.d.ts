import type { AliasResolver } from "@digitak/grubber/utilities/resolveAliases"
export type { AliasResolver }

export declare function build(aliases?: Array<AliasResolver>): void
export declare function compile(): void
export declare function patch(aliases?: Array<AliasResolver>): void

export declare function deepEqual<T>(a: T, b: T): boolean;
export declare function getHeaders(): Record<string, string> | undefined;
export declare function digestDir(path: string, conf: string): Promise<string>;
export declare function generateHash(content: string): Promise<string>;
export declare function generateHashFromBuffer(content: BufferSource): Promise<string>;
export declare function readInlinePathSync(path: string): string;
export declare function sleep(ms: number): Promise<unknown>;
//# sourceMappingURL=utils.d.ts.map
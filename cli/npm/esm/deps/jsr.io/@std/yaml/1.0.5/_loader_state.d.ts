import { type Schema, type TypeMap } from "./_schema.js";
import type { Type } from "./_type.js";
export interface LoaderStateOptions {
    /** specifies a schema to use. */
    schema?: Schema;
    /** compatibility with JSON.parse behaviour. */
    allowDuplicateKeys?: boolean;
    /** function to call on warning messages. */
    onWarning?(error: Error): void;
}
export declare class LoaderState {
    #private;
    input: string;
    length: number;
    lineIndent: number;
    lineStart: number;
    position: number;
    line: number;
    onWarning: ((error: Error) => void) | undefined;
    allowDuplicateKeys: boolean;
    implicitTypes: Type<"scalar">[];
    typeMap: TypeMap;
    version: string | null;
    checkLineBreaks: boolean;
    tagMap: Map<any, any>;
    anchorMap: Map<any, any>;
    tag: string | null | undefined;
    anchor: string | null | undefined;
    kind: string | null | undefined;
    result: unknown[] | Record<string, unknown> | string | null;
    constructor(input: string, { schema, onWarning, allowDuplicateKeys, }: LoaderStateOptions);
    readIndent(): void;
    peek(offset?: number): number;
    next(): number;
    throwError(message: string): never;
    dispatchWarning(message: string): void;
    yamlDirectiveHandler(...args: string[]): void;
    tagDirectiveHandler(...args: string[]): undefined;
    captureSegment(start: number, end: number, checkJson: boolean): undefined;
    readBlockSequence(nodeIndent: number): boolean;
    mergeMappings(destination: Record<string, unknown>, source: Record<string, unknown>, overridableKeys: Set<string>): undefined;
    storeMappingPair(result: Record<string, unknown>, overridableKeys: Set<string>, keyTag: string | null, keyNode: Record<PropertyKey, unknown> | unknown[] | string | null, valueNode: unknown, startLine?: number, startPos?: number): Record<string, unknown>;
    readLineBreak(): undefined;
    skipSeparationSpace(allowComments: boolean, checkIndent: number): number;
    testDocumentSeparator(): boolean;
    writeFoldedLines(count: number): void;
    readPlainScalar(nodeIndent: number, withinFlowCollection: boolean): boolean;
    readSingleQuotedScalar(nodeIndent: number): boolean;
    readDoubleQuotedScalar(nodeIndent: number): boolean;
    readFlowCollection(nodeIndent: number): boolean;
    readBlockScalar(nodeIndent: number): boolean;
    readBlockMapping(nodeIndent: number, flowIndent: number): boolean;
    readTagProperty(): boolean;
    readAnchorProperty(): boolean;
    readAlias(): boolean;
    composeNode(parentIndent: number, nodeContext: number, allowToSeek: boolean, allowCompact: boolean): boolean;
    readDocument(): string | Record<string, unknown> | unknown[] | null;
    readDocuments(): Generator<string | Record<string, unknown> | unknown[] | null, void, unknown>;
}
//# sourceMappingURL=_loader_state.d.ts.map
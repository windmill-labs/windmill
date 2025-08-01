import { IDiagnostic } from './diagnostics';
export declare type SourceMapParser<T = unknown, A extends object = object, L = unknown> = (value: string) => IParserResult<T, A, L>;
export declare type DocumentUri = string;
export declare type Segment = string | number;
export declare type JsonPath = Segment[];
export interface IParserResult<T = any, A extends object = object, L = unknown, M = unknown> extends IParserASTResult<T, A, L, M> {
    diagnostics: IDiagnostic[];
}
export interface IParserASTResult<T = unknown, A extends object = object, L = unknown, M = unknown> {
    data: T;
    ast: A;
    lineMap: L;
    metadata?: M;
}
export declare type GetJsonPathForPosition<R extends IParserASTResult> = (result: R, position: IPosition) => JsonPath | undefined;
export declare type GetLocationForJsonPath<R extends IParserASTResult> = (result: R, path: JsonPath, closest?: boolean) => ILocation | undefined;
export interface IPosition {
    /**
     * Line position in a document (zero-based).
     */
    readonly line: number;
    /**
     * Character offset on a line in a document (zero-based). Assuming that the line is
     * represented as a string, the `character` value represents the gap between the
     * `character` and `character + 1`.
     *
     * If the character value is greater than the line length it defaults back to the
     * line length.
     */
    readonly character: number;
}
/**
 * A range represents an ordered pair of two positions.
 * It is guaranteed that start isBeforeOrEqual end.
 */
export interface IRange {
    /**
     * The start position. It is before or equal to end.
     */
    readonly start: IPosition;
    /**
     * The end position. It is after or equal to start.
     */
    readonly end: IPosition;
}
export interface ILocation {
    uri?: DocumentUri;
    range: IRange;
}
export interface IJsonLocation extends ILocation {
    path: JsonPath;
}

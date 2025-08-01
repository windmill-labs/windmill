import { ILocation, IRange, JsonPath } from './parsers';
/**
 * Represents the severity of diagnostics.
 */
export declare enum DiagnosticSeverity {
    /**
     * Something not allowed by the rules of a language or other means.
     */
    Error = 0,
    /**
     * Something suspicious but allowed.
     */
    Warning = 1,
    /**
     * Something to inform about but not a problem.
     */
    Information = 2,
    /**
     * Something to hint to a better way of doing it, like proposing
     * a refactoring.
     */
    Hint = 3
}
export declare type DiagnosticTag = string;
export interface IDiagnostic {
    /**
     * The range to which this diagnostic applies.
     */
    range: IRange;
    /**
     * The human-readable message.
     */
    message: string;
    /**
     * The severity, default is error.
     */
    severity: DiagnosticSeverity;
    /**
     * A human-readable string describing the source of this
     * diagnostic, e.g. 'typescript' or 'super lint'.
     */
    source?: string;
    /**
     * The JSONPath pointing to property to which this diagnostic applies.
     */
    path?: JsonPath;
    /**
     * A code or identifier for this diagnostics. Will not be surfaced
     * to the user, but should be used for later processing, e.g. when
     * providing code actions.
     */
    code?: string | number;
    /**
     * Additional metadata about the diagnostic.
     */
    tags?: DiagnosticTag[];
    /**
     * An array of related diagnostic information, e.g. when symbol-names within
     * a scope collide all definitions can be marked via this property.
     */
    relatedInformation?: IDiagnosticRelatedInformation[];
}
/**
 * Represents a related message and source code location for a diagnostic. This should be
 * used to point to code locations that cause or related to a diagnostics, e.g when duplicating
 * a symbol in a scope.
 */
export interface IDiagnosticRelatedInformation {
    /**
     * The location of this related diagnostic information.
     */
    location: ILocation;
    /**
     * The message of this related diagnostic information.
     */
    message: string;
}

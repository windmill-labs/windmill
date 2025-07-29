/**
 * Type alias for enum values - can be an array of strings or undefined
 */
export type EnumType = string[] | undefined;
/**
 * Represents a property in a JSON schema with various validation and display options
 */
export interface SchemaProperty {
    type: string | undefined;
    description?: string;
    pattern?: string;
    default?: any;
    enum?: EnumType;
    contentEncoding?: 'base64' | 'binary';
    format?: string;
    items?: {
        type?: 'string' | 'number' | 'bytes' | 'object' | 'resource';
        contentEncoding?: 'base64';
        enum?: string[];
        resourceType?: string;
        properties?: {
            [name: string]: SchemaProperty;
        };
    };
    min?: number;
    max?: number;
    currency?: string;
    currencyLocale?: string;
    multiselect?: boolean;
    customErrorMessage?: string;
    properties?: {
        [name: string]: SchemaProperty;
    };
    required?: string[];
    showExpr?: string;
    password?: boolean;
    order?: string[];
    nullable?: boolean;
    dateFormat?: string;
    title?: string;
    placeholder?: string;
    oneOf?: SchemaProperty[];
    originalType?: string;
}
/**
 * Converts argument signature types to JSON schema properties.
 * This function handles various Windmill-specific types and converts them
 * to standard JSON schema format while preserving existing property metadata.
 *
 * @param t - The argument signature type definition (can be string or complex object types)
 * @param oldS - Existing schema property to update with new type information
 */
export declare function argSigToJsonSchemaType(t: string | {
    resource: string | null;
} | {
    list: (string | {
        name?: string;
        props?: {
            key: string;
            typ: any;
        }[];
    }) | {
        str: any;
    } | {
        object: {
            name?: string;
            props?: {
                key: string;
                typ: any;
            }[];
        };
    } | null;
} | {
    dynselect: string;
} | {
    str: string[] | null;
} | {
    object: {
        name?: string;
        props?: {
            key: string;
            typ: any;
        }[];
    };
} | {
    oneof: {
        label: string;
        properties: {
            key: string;
            typ: any;
        }[];
    }[];
}, oldS: SchemaProperty): void;

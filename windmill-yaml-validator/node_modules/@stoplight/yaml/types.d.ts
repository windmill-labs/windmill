import { IParserResult, Optional } from '@stoplight/types';
import * as YAMLAstParser from '@stoplight/yaml-ast-parser';
import { DumpOptions, Kind, ScalarType } from '@stoplight/yaml-ast-parser';
export interface IParseOptions extends YAMLAstParser.LoadOptions {
    json?: boolean;
    bigInt?: boolean;
    mergeKeys?: boolean;
    preserveKeyOrder?: boolean;
    attachComments?: boolean;
}
export declare type YAMLBaseNode<K extends Kind> = Omit<YAMLAstParser.YAMLNode, 'kind' | 'parent'> & {
    kind: K;
    parent: YAMLNode;
};
export declare type YAMLAnchorReference = Omit<YAMLAstParser.YAMLAnchorReference, 'kind' | 'value' | 'parent'> & {
    kind: Kind.ANCHOR_REF;
    value: Optional<YAMLNode>;
    parent: YAMLNode;
};
export declare type YAMLIncludeReference = YAMLBaseNode<Kind.INCLUDE_REF>;
export declare type YAMLScalar = Omit<YAMLAstParser.YAMLScalar, 'kind' | 'parent'> & {
    kind: Kind.SCALAR;
    parent: YAMLNode;
    valueObject: unknown;
};
export declare type YAMLMap = Omit<YAMLAstParser.YamlMap, 'kind' | 'mappings' | 'parent'> & {
    kind: Kind.MAP;
    mappings: YAMLMapping[];
    parent: YAMLNode;
};
export declare type YAMLMapping = Omit<YAMLAstParser.YAMLMapping, 'kind' | 'key' | 'value' | 'parent'> & {
    kind: Kind.MAPPING;
    key: YAMLScalar;
    value: YAMLNode | null;
    parent: YAMLNode;
};
export declare type YAMLSequence = Omit<YAMLAstParser.YAMLSequence, 'kind' | 'items' | 'parent'> & {
    kind: Kind.SEQ;
    items: Array<YAMLNode | null>;
    parent: YAMLNode;
};
export declare type YAMLNode = YAMLAnchorReference | YAMLIncludeReference | YAMLScalar | YAMLMap | YAMLMapping | YAMLSequence;
export declare type YamlComments = NonNullable<DumpOptions['comments']>;
export declare type YamlParserResult<T> = IParserResult<T, YAMLNode, number[], IParseOptions> & {
    comments: YamlComments;
};
export { Kind, ScalarType };

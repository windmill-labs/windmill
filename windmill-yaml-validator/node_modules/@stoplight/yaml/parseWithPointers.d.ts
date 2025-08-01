import { IParseOptions, YamlParserResult } from './types';
export declare const parseWithPointers: <T>(value: string, options?: IParseOptions | undefined) => YamlParserResult<T | undefined>;

import type { ScriptLang } from "../gen/types.gen.ts";

export type EnumType = string[] | undefined;

export type Schema = {
  $schema: string | undefined;
  type: string;
  "x-windmill-dyn-select-code"?: string;
  "x-windmill-dyn-select-lang"?: ScriptLang;
  properties: { [name: string]: SchemaProperty };
  order?: string[];
  required: string[];
};

export interface SchemaProperty {
  type: string | undefined;
  description?: string;
  pattern?: string;
  default?: any;
  enum?: EnumType;
  contentEncoding?: "base64" | "binary";
  format?: string;
  items?: {
    type?: "string" | "number" | "bytes" | "object" | "resource";
    contentEncoding?: "base64";
    enum?: string[];
    resourceType?: string;
    properties?: { [name: string]: SchemaProperty };
  };
  min?: number;
  max?: number;
  currency?: string;
  currencyLocale?: string;
  multiselect?: boolean;
  customErrorMessage?: string;
  properties?: { [name: string]: SchemaProperty };
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

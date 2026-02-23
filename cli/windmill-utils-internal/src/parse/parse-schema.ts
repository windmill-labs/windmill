/**
 * Type alias for enum values - can be an array of strings or undefined
 */
export type EnumType =
  | string[]
  | { label: string; value: string }[]
  | undefined;

/**
 * Represents a property in a JSON schema with various validation and display options
 */
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
    enum?: EnumType;
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

const ITEMS_PRESERVED_FIELDS = [
  "properties",
  "required",
  "additionalProperties",
  "enum",
  "resourceType",
  "contentEncoding",
  "description",
] as (keyof SchemaProperty["items"])[];

/**
 * Converts argument signature types to JSON schema properties.
 * This function handles various Windmill-specific types and converts them
 * to standard JSON schema format while preserving existing property metadata.
 *
 * @param t - The argument signature type definition (can be string or complex object types)
 * @param oldS - Existing schema property to update with new type information
 */

export function argSigToJsonSchemaType(
  t:
    | string
    | { resource: string | null }
    | {
        list:
          | (string | { name?: string; props?: { key: string; typ: any }[] })
          | { str: any }
          | { object: { name?: string; props?: { key: string; typ: any }[] } }
          | null;
      }
    | { dynselect: string }
    | { dynmultiselect: string }
    | { str: string[] | null }
    | { object: { name?: string; props?: { key: string; typ: any }[] } }
    | {
        oneof: {
          label: string;
          properties: { key: string; typ: any }[];
        }[];
      },
  oldS: SchemaProperty
): void {
  const newS: SchemaProperty = { type: "" };
  if (t === "int") {
    newS.type = "integer";
  } else if (t === "float") {
    newS.type = "number";
  } else if (t === "bool") {
    newS.type = "boolean";
  } else if (t === "email") {
    newS.type = "string";
    newS.format = "email";
  } else if (t === "sql") {
    newS.type = "string";
    newS.format = "sql";
  } else if (t === "yaml") {
    newS.type = "string";
    newS.format = "yaml";
  } else if (t === "bytes") {
    newS.type = "string";
    newS.contentEncoding = "base64";
    newS.originalType = "bytes";
  } else if (t === "datetime") {
    newS.type = "string";
    newS.format = "date-time";
  } else if (t === "date") {
    newS.type = "string";
    newS.format = "date";
  } else if (typeof t !== "string" && "oneof" in t) {
    newS.type = "object";
    if (t.oneof) {
      newS.oneOf = t.oneof.map((obj) => {
        const oldObjS =
          oldS.oneOf?.find((o) => o?.title === obj.label) ?? undefined;
        const properties: Record<string, any> = {};
        for (const prop of obj.properties) {
          if (oldObjS?.properties && prop.key in oldObjS?.properties) {
            properties[prop.key] = oldObjS?.properties[prop.key];
          } else {
            properties[prop.key] = { description: "", type: "" };
          }
          argSigToJsonSchemaType(prop.typ, properties[prop.key]);
        }
        return {
          type: "object",
          title: obj.label,
          properties,
          order: oldObjS?.order ?? undefined,
        };
      });
    }
  } else if (typeof t !== "string" && `object` in t) {
    newS.type = "object";
    if (t.object.name) {
      newS.format = `resource-${t.object.name}`;
    }
    if (t.object.props) {
      const properties: Record<string, any> = {};
      for (const prop of t.object.props) {
        if (oldS.properties && prop.key in oldS.properties) {
          properties[prop.key] = oldS.properties[prop.key];
        } else {
          properties[prop.key] = { description: "", type: "" };
        }
        argSigToJsonSchemaType(prop.typ, properties[prop.key]);
      }
      newS.properties = properties;
    }
  } else if (typeof t !== "string" && `str` in t) {
    newS.type = "string";
    if (t.str) {
      newS.originalType = "enum";
      newS.enum = t.str;
    } else if (oldS.originalType == "string" && oldS.enum) {
      newS.originalType = "string";
      newS.enum = oldS.enum;
    } else {
      newS.originalType = "string";
      newS.enum = undefined;
    }
  } else if (typeof t !== "string" && `resource` in t) {
    newS.type = "object";
    newS.format = `resource-${t.resource}`;
  } else if (typeof t !== "string" && `dynselect` in t) {
    newS.type = "object";
    newS.format = `dynselect-${t.dynselect}`;
  } else if (typeof t !== "string" && `dynmultiselect` in t) {
    newS.type = "object";
    newS.format = `dynmultiselect-${t.dynmultiselect}`;
  } else if (typeof t !== "string" && `list` in t) {
    newS.type = "array";
    if (t.list === "int" || t.list === "float") {
      newS.items = { type: "number" };
      newS.originalType = "number[]";
    } else if (t.list === "bytes") {
      newS.items = { type: "string", contentEncoding: "base64" };
      newS.originalType = "bytes[]";
    } else if (
      t.list &&
      typeof t.list == "object" &&
      "str" in t.list &&
      t.list.str
    ) {
      newS.items = { type: "string", enum: t.list.str };
      newS.originalType = "enum[]";
    } else if (
      t.list == "string" ||
      (t.list && typeof t.list == "object" && "str" in t.list)
    ) {
      newS.items = { type: "string", enum: oldS.items?.enum };
      newS.originalType = "string[]";
    } else if (
      t.list &&
      typeof t.list == "object" &&
      "resource" in t.list &&
      t.list.resource
    ) {
      newS.items = {
        type: "resource",
        resourceType: t.list.resource as string,
      };
      newS.originalType = "resource[]";
    } else if (
      t.list &&
      typeof t.list == "object" &&
      "object" in t.list &&
      t.list.object
    ) {
      if (t.list.object.name) {
        newS.format = `resource-${t.list.object.name}`;
      }
      if (t.list.object.props && t.list.object.props.length > 0) {
        const properties: Record<string, any> = {};
        for (const prop of t.list.object.props) {
          properties[prop.key] = { description: "", type: "" };
          argSigToJsonSchemaType(prop.typ, properties[prop.key]);
        }
        newS.items = { type: "object", properties: properties };
      } else {
        // Preserve ALL user-defined fields when parser cannot infer structure
        newS.items = { type: oldS.items?.type || "object" };

        if (oldS.items && typeof oldS.items === "object") {
          ITEMS_PRESERVED_FIELDS.forEach((field) => {
            if (oldS.items && oldS.items[field] !== undefined) {
              (newS.items as any)[field] = oldS.items[field];
            }
          });
        }
      }
      newS.originalType = "record[]";
    } else {
      // Preserve ALL user-defined fields for untyped lists (same as record[] branch)
      newS.items = { type: oldS.items?.type || "object" };

      if (oldS.items && typeof oldS.items === "object") {
        ITEMS_PRESERVED_FIELDS.forEach((field) => {
          if (oldS.items && oldS.items[field] !== undefined) {
            (newS.items as any)[field] = oldS.items[field];
          }
        });
      }
      newS.originalType = "object[]";
    }
  } else {
    // Preserve existing type when inference fails, default to "object" for undefined/null
    newS.type = oldS.type ?? "object";
  }

  const preservedFields = [
    "description",
    "pattern",
    "min",
    "max",
    "currency",
    "currencyLocale",
    "multiselect",
    "customErrorMessage",
    "required",
    "showExpr",
    "password",
    "order",
    "dateFormat",
    "title",
    "placeholder",
  ];

  preservedFields.forEach((field) => {
    // @ts-ignore
    if (oldS[field] !== undefined) {
      // @ts-ignore
      newS[field] = oldS[field];
    }
  });

  if (oldS.type != newS.type) {
    for (const prop of Object.getOwnPropertyNames(newS)) {
      if (prop != "description") {
        // @ts-ignore
        delete oldS[prop];
      }
    }
  } else if (
    (oldS.format == "date" || oldS.format === "date-time") &&
    newS.format == "string"
  ) {
    newS.format = oldS.format;
  } else if (newS.format == "date-time" && oldS.format == "date") {
    newS.format = "date";
  } else if (newS.format == "date" || newS.format == "date-time") {
    newS.format = oldS.format;
  } else if (oldS.items?.type != newS.items?.type) {
    delete oldS.items;
  } else if (oldS.type == "string" && oldS.format != undefined) {
    newS.format = oldS.format;
  }

  if (
    (oldS.type != newS.type || newS.type != "string") &&
    newS.format == undefined
  ) {
    oldS.format = undefined;
  }

  Object.assign(oldS, newS);
}

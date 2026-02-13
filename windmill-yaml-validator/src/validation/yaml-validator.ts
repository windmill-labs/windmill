import Ajv, { AnySchema, ErrorObject, ValidateFunction } from "ajv";
import { parseWithPointers, YamlParserResult } from "@stoplight/yaml";
import openFlowSchema from "../gen/openflow.json";
import scheduleSchema from "../gen/schedule.json";
import gcpTriggerSchema from "../gen/triggers/gcp.json";
import httpTriggerSchema from "../gen/triggers/http.json";
import kafkaTriggerSchema from "../gen/triggers/kafka.json";
import mqttTriggerSchema from "../gen/triggers/mqtt.json";
import natsTriggerSchema from "../gen/triggers/nats.json";
import postgresTriggerSchema from "../gen/triggers/postgres.json";
import sqsTriggerSchema from "../gen/triggers/sqs.json";
import websocketTriggerSchema from "../gen/triggers/websocket.json";
import emailTriggerSchema from "../gen/triggers/email.json";

export const SUPPORTED_TRIGGER_KINDS = [
  "http",
  "websocket",
  "kafka",
  "nats",
  "postgres",
  "mqtt",
  "sqs",
  "gcp",
  "email",
] as const;

export type TriggerKind = (typeof SUPPORTED_TRIGGER_KINDS)[number];

export type ValidationTarget =
  | { type: "flow" }
  | { type: "schedule" }
  | { type: "trigger"; triggerKind: TriggerKind };

const TRIGGER_SCHEMAS: Record<TriggerKind, AnySchema> = {
  http: httpTriggerSchema as AnySchema,
  websocket: websocketTriggerSchema as AnySchema,
  kafka: kafkaTriggerSchema as AnySchema,
  nats: natsTriggerSchema as AnySchema,
  postgres: postgresTriggerSchema as AnySchema,
  mqtt: mqttTriggerSchema as AnySchema,
  sqs: sqsTriggerSchema as AnySchema,
  gcp: gcpTriggerSchema as AnySchema,
  email: emailTriggerSchema as AnySchema,
};

/**
 * Infers validation target from file name conventions used by Windmill sync.
 */
export function getValidationTargetFromFilename(
  filePath: string
): ValidationTarget | null {
  const path = filePath.toLowerCase();

  if (/[/\\]flow\.ya?ml$/.test(path) || /^flow\.ya?ml$/.test(path)) {
    return { type: "flow" };
  }

  if (/\.schedule\.ya?ml$/.test(path)) {
    return { type: "schedule" };
  }

  const triggerMatch = path.match(
    /\.(http|websocket|kafka|nats|postgres|mqtt|sqs|gcp|email)_trigger\.ya?ml$/
  );
  if (triggerMatch) {
    return {
      type: "trigger",
      triggerKind: triggerMatch[1] as TriggerKind,
    };
  }

  return null;
}

/**
 * Unified YAML validator for Windmill flow, schedule, and trigger files.
 */
export class WindmillYamlValidator {
  private readonly validateFlow: ValidateFunction;
  private readonly validateSchedule: ValidateFunction;
  private readonly validateTrigger: Record<TriggerKind, ValidateFunction>;

  constructor() {
    const ajv = new Ajv({
      strict: false,
      allErrors: true,
      discriminator: true,
      validateFormats: false,
    });

    for (const [name, schema] of Object.entries(openFlowSchema.components.schemas)) {
      ajv.addSchema(schema as AnySchema, `#/components/schemas/${name}`);
    }

    this.validateFlow = ajv.getSchema("#/components/schemas/OpenFlow")!;
    this.validateSchedule = ajv.compile(scheduleSchema as AnySchema);

    this.validateTrigger = Object.fromEntries(
      SUPPORTED_TRIGGER_KINDS.map((kind) => [kind, ajv.compile(TRIGGER_SCHEMAS[kind])])
    ) as Record<TriggerKind, ValidateFunction>;
  }

  /**
   * Validates a Windmill YAML document based on the selected target.
   * @param doc - The YAML document as string
   * @param target - Which Windmill schema to validate against
   */
  validate(
    doc: string,
    target: ValidationTarget
  ): { parsed: YamlParserResult<unknown>; errors: ErrorObject[] } {
    if (typeof doc !== "string") {
      throw new Error("Document must be a string");
    }

    const parsed = parseWithPointers(doc);
    const { data } = parsed;

    let validator: ValidateFunction;
    if (target.type === "flow") {
      validator = this.validateFlow;
    } else if (target.type === "schedule") {
      validator = this.validateSchedule;
    } else {
      validator = this.validateTrigger[target.triggerKind];
      if (!validator) {
        throw new Error(`Unsupported trigger kind: ${target.triggerKind}`);
      }
    }

    const ok = validator(data);
    if (ok) {
      return { parsed, errors: [] };
    }

    return {
      parsed,
      errors: validator.errors || [],
    };
  }
}

import {
  getValidationTargetFromFilename,
  TriggerKind,
  WindmillYamlValidator,
} from "../yaml-validator";

describe("WindmillYamlValidator resource validation", () => {
  let validator: WindmillYamlValidator;

  beforeEach(() => {
    validator = new WindmillYamlValidator();
  });

  it("validates a valid schedule file", () => {
    const schedule = {
      schedule: "0 0 12 * * *",
      timezone: "UTC",
      enabled: true,
      script_path: "f/jobs/daily_sync",
      is_flow: false,
      args: { dry_run: true },
    };

    const result = validator.validate(JSON.stringify(schedule), {
      type: "schedule",
    });

    expect(result.errors).toHaveLength(0);
  });

  it("returns required-field errors for invalid schedules", () => {
    const invalidSchedule = {
      timezone: "UTC",
      enabled: true,
      script_path: "f/jobs/daily_sync",
      is_flow: false,
    };

    const result = validator.validate(JSON.stringify(invalidSchedule), {
      type: "schedule",
    });

    expect(result.errors.length).toBeGreaterThan(0);
    expect(
      result.errors.some(
        (error) =>
          error.keyword === "required" &&
          (error.params as { missingProperty?: string })?.missingProperty ===
            "schedule"
      )
    ).toBe(true);
  });

  describe("trigger schemas", () => {
    const validTriggers: Record<TriggerKind, Record<string, unknown>> = {
      http: {
        script_path: "f/triggers/http_handler",
        is_flow: false,
        route_path: "api/webhook",
        request_type: "sync",
        authentication_method: "none",
        http_method: "post",
        is_static_website: false,
        workspaced_route: false,
        wrap_body: false,
        raw_string: false,
      },
      websocket: {
        script_path: "f/triggers/ws_handler",
        is_flow: false,
        url: "wss://example.com/socket",
        filters: [],
        can_return_message: false,
        can_return_error_result: true,
      },
      kafka: {
        script_path: "f/triggers/kafka_handler",
        is_flow: false,
        kafka_resource_path: "f/resources/kafka",
        group_id: "group-a",
        topics: ["topic-a"],
        filters: [],
      },
      nats: {
        script_path: "f/triggers/nats_handler",
        is_flow: false,
        nats_resource_path: "f/resources/nats",
        use_jetstream: false,
        subjects: ["events.>"],
      },
      postgres: {
        script_path: "f/triggers/postgres_handler",
        is_flow: false,
        postgres_resource_path: "f/resources/postgres",
        publication_name: "pub_main",
        replication_slot_name: "slot_main",
      },
      mqtt: {
        script_path: "f/triggers/mqtt_handler",
        is_flow: false,
        mqtt_resource_path: "f/resources/mqtt",
        subscribe_topics: [],
      },
      sqs: {
        script_path: "f/triggers/sqs_handler",
        is_flow: false,
        queue_url: "https://sqs.us-east-1.amazonaws.com/12345/my-queue",
        aws_resource_path: "f/resources/aws",
        aws_auth_resource_type: "credentials",
      },
      gcp: {
        script_path: "f/triggers/gcp_handler",
        is_flow: false,
        gcp_resource_path: "f/resources/gcp",
        topic_id: "topic-a",
        subscription_id: "sub-a",
        delivery_type: "pull",
        subscription_mode: "existing",
      },
    };

    const missingRequiredField: Record<TriggerKind, string> = {
      http: "request_type",
      websocket: "url",
      kafka: "topics",
      nats: "subjects",
      postgres: "publication_name",
      mqtt: "mqtt_resource_path",
      sqs: "queue_url",
      gcp: "topic_id",
    };

    for (const [kind, validDocument] of Object.entries(validTriggers) as [
      TriggerKind,
      Record<string, unknown>,
    ][]) {
      it(`validates a valid ${kind} trigger file`, () => {
        const result = validator.validate(JSON.stringify(validDocument), {
          type: "trigger",
          triggerKind: kind,
        });

        expect(result.errors).toHaveLength(0);
      });

      it(`returns required-field errors for invalid ${kind} trigger file`, () => {
        const missingField = missingRequiredField[kind];
        const invalidDocument = { ...validDocument };
        delete invalidDocument[missingField];

        const result = validator.validate(JSON.stringify(invalidDocument), {
          type: "trigger",
          triggerKind: kind,
        });

        expect(result.errors.length).toBeGreaterThan(0);
        expect(
          result.errors.some(
            (error) =>
              error.keyword === "required" &&
              (error.params as { missingProperty?: string })?.missingProperty ===
                missingField
          )
        ).toBe(true);
      });
    }
  });

  it("throws for unsupported trigger kinds", () => {
    expect(() =>
      validator.validate("{}", {
        type: "trigger",
        triggerKind: "email" as any,
      })
    ).toThrow("Unsupported trigger kind: email");
  });
});

describe("getValidationTargetFromFilename", () => {
  it("detects flow files", () => {
    expect(getValidationTargetFromFilename("f/my.flow/flow.yaml")).toEqual({
      type: "flow",
    });
    expect(getValidationTargetFromFilename("flow.yml")).toEqual({
      type: "flow",
    });
  });

  it("detects schedule files", () => {
    expect(
      getValidationTargetFromFilename("f/folder/daily.schedule.yaml")
    ).toEqual({
      type: "schedule",
    });
  });

  it("detects trigger files", () => {
    expect(
      getValidationTargetFromFilename("u/user/handler.http_trigger.yaml")
    ).toEqual({
      type: "trigger",
      triggerKind: "http",
    });
    expect(
      getValidationTargetFromFilename("f/sub/event.kafka_trigger.yml")
    ).toEqual({
      type: "trigger",
      triggerKind: "kafka",
    });
  });

  it("returns null for unsupported trigger kinds and unknown files", () => {
    expect(
      getValidationTargetFromFilename("u/user/mail.email_trigger.yaml")
    ).toBeNull();
    expect(getValidationTargetFromFilename("README.md")).toBeNull();
  });
});

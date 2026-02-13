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

  // ── Schedules ──────────────────────────────────────────────────────────

  describe("schedules", () => {
    it("validates a minimal schedule", () => {
      const result = validator.validate(
        JSON.stringify({
          schedule: "0 0 12 * * *",
          timezone: "UTC",
          enabled: true,
          script_path: "f/jobs/daily_sync",
          is_flow: false,
        }),
        { type: "schedule" }
      );
      expect(result.errors).toHaveLength(0);
    });

    it("validates a fully-configured schedule with handlers and retry", () => {
      const result = validator.validate(
        JSON.stringify({
          schedule: "0 */5 * * * *",
          timezone: "Europe/Paris",
          enabled: true,
          script_path: "f/jobs/full_sync",
          is_flow: true,
          args: { batch_size: 100, dry_run: false },
          on_failure: "f/handlers/on_fail",
          on_failure_times: 3,
          on_failure_exact: true,
          on_failure_extra_args: { notify: true },
          on_recovery: "f/handlers/on_recover",
          on_recovery_times: 2,
          on_recovery_extra_args: { channel: "#ops" },
          on_success: "f/handlers/on_success",
          on_success_extra_args: { log: true },
          ws_error_handler_muted: true,
          retry: {
            constant: { attempts: 3, seconds: 10 },
            exponential: {
              attempts: 5,
              multiplier: 2,
              seconds: 1,
              random_factor: 50,
            },
            retry_if: { expr: "error.message.includes('timeout')" },
          },
          summary: "Full sync every 5 minutes",
          description: "Runs the full synchronization pipeline",
          no_flow_overlap: true,
          tag: "heavy",
          paused_until: "2026-03-01T00:00:00Z",
          cron_version: "v2",
          dynamic_skip: "f/helpers/should_skip",
        }),
        { type: "schedule" }
      );
      expect(result.errors).toHaveLength(0);
    });

    it("accepts null for all nullable optional fields", () => {
      const result = validator.validate(
        JSON.stringify({
          schedule: "0 0 12 * * *",
          timezone: "UTC",
          enabled: true,
          script_path: "f/jobs/daily_sync",
          is_flow: false,
          args: null,
          on_failure: null,
          tag: null,
          retry: null,
          paused_until: null,
          summary: null,
          description: null,
          cron_version: null,
          dynamic_skip: null,
        }),
        { type: "schedule" }
      );
      expect(result.errors).toHaveLength(0);
    });

    it("rejects a schedule missing required fields", () => {
      const result = validator.validate(
        JSON.stringify({ timezone: "UTC" }),
        { type: "schedule" }
      );
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it("rejects wrong types in schedule fields", () => {
      const result = validator.validate(
        JSON.stringify({
          schedule: 12345,
          timezone: "UTC",
          enabled: "yes",
          script_path: "f/jobs/daily_sync",
          is_flow: "true",
        }),
        { type: "schedule" }
      );
      // Should catch schedule (not string), enabled (not boolean), is_flow (not boolean)
      expect(result.errors.length).toBeGreaterThanOrEqual(3);
    });

    it("rejects invalid retry constraints", () => {
      const result = validator.validate(
        JSON.stringify({
          schedule: "0 0 12 * * *",
          timezone: "UTC",
          enabled: true,
          script_path: "f/jobs/daily_sync",
          is_flow: false,
          retry: {
            exponential: { attempts: 3, seconds: 0, random_factor: 150 },
            retry_if: {},
          },
        }),
        { type: "schedule" }
      );
      // seconds < 1, random_factor > 100, retry_if missing expr
      expect(result.errors.length).toBeGreaterThanOrEqual(3);
    });
  });

  // ── Triggers — valid minimal + valid with missing required ─────────────

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
      email: {
        script_path: "f/triggers/email_handler",
        is_flow: false,
        local_part: "inbox",
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
      email: "local_part",
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
        triggerKind: "foobar" as any,
      })
    ).toThrow("Unsupported trigger kind: foobar");
  });

  // ── Triggers — realistic full documents with all optional fields ───────

  describe("fully-configured triggers", () => {
    it("validates http trigger with static assets, auth, error handling and retry", () => {
      const result = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/http_handler",
          is_flow: false,
          route_path: "api/webhook",
          request_type: "sync_sse",
          authentication_method: "api_key",
          http_method: "put",
          is_static_website: true,
          workspaced_route: true,
          wrap_body: true,
          raw_string: false,
          summary: "Incoming webhook",
          description: "Handles external webhook deliveries",
          authentication_resource_path: "f/resources/api_key_config",
          static_asset_config: { s3: "my-bucket/assets", filename: "index.html" },
          error_handler_path: "f/handlers/on_error",
          error_handler_args: { notify: true },
          retry: {
            constant: { attempts: 2, seconds: 5 },
            retry_if: { expr: "error.status === 429" },
          },
        }),
        { type: "trigger", triggerKind: "http" }
      );
      expect(result.errors).toHaveLength(0);
    });

    it("validates websocket trigger with initial messages and filters", () => {
      const result = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/ws_handler",
          is_flow: true,
          url: "wss://stream.example.com/v1",
          filters: [{ key: "event", value: "trade" }],
          can_return_message: true,
          can_return_error_result: false,
          initial_messages: [
            { raw_message: '{"action":"subscribe","channel":"trades"}' },
            {
              runnable_result: {
                path: "f/helpers/ws_auth",
                args: { token: "abc" },
                is_flow: false,
              },
            },
          ],
          url_runnable_args: { env: "production" },
          error_handler_path: "f/handlers/on_error",
          retry: { exponential: { attempts: 5, multiplier: 2, seconds: 1, random_factor: 25 } },
        }),
        { type: "trigger", triggerKind: "websocket" }
      );
      expect(result.errors).toHaveLength(0);
    });

    it("validates mqtt trigger with v5 config and subscribe topics", () => {
      const result = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/mqtt_handler",
          is_flow: false,
          mqtt_resource_path: "f/resources/mqtt",
          subscribe_topics: [
            { topic: "sensor/+/data", qos: "qos1" },
            { topic: "alerts/#", qos: "qos2" },
          ],
          client_version: "v5",
          client_id: "windmill-consumer-1",
          v5_config: {
            clean_start: true,
            topic_alias_maximum: 10,
            session_expiry_interval: 300,
          },
          error_handler_path: "f/handlers/on_error",
          retry: { constant: { attempts: 3, seconds: 10 } },
        }),
        { type: "trigger", triggerKind: "mqtt" }
      );
      expect(result.errors).toHaveLength(0);
    });

    it("validates nats trigger with jetstream config", () => {
      const result = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/nats_handler",
          is_flow: true,
          nats_resource_path: "f/resources/nats",
          use_jetstream: true,
          subjects: ["orders.>"],
          stream_name: "ORDERS",
          consumer_name: "windmill-consumer",
          error_handler_path: "f/handlers/on_error",
          error_handler_args: {},
          retry: { constant: { attempts: 5, seconds: 30 } },
        }),
        { type: "trigger", triggerKind: "nats" }
      );
      expect(result.errors).toHaveLength(0);
    });

    it("validates sqs trigger with message attributes and oidc auth", () => {
      const result = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/sqs_handler",
          is_flow: false,
          queue_url: "https://sqs.us-east-1.amazonaws.com/12345/my-queue",
          aws_resource_path: "f/resources/aws",
          aws_auth_resource_type: "oidc",
          message_attributes: ["traceId", "source"],
          error_handler_path: "f/handlers/on_error",
        }),
        { type: "trigger", triggerKind: "sqs" }
      );
      expect(result.errors).toHaveLength(0);
    });

    it("validates gcp trigger with push delivery config", () => {
      const result = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/gcp_handler",
          is_flow: false,
          gcp_resource_path: "f/resources/gcp",
          topic_id: "topic-a",
          subscription_id: "sub-push",
          delivery_type: "push",
          subscription_mode: "create_update",
          delivery_config: {
            authenticate: true,
            base_endpoint: "https://app.example.com/webhook",
            audience: "https://app.example.com",
          },
          error_handler_path: "f/handlers/on_error",
          retry: { retry_if: { expr: "error.message.includes('quota')" } },
        }),
        { type: "trigger", triggerKind: "gcp" }
      );
      expect(result.errors).toHaveLength(0);
    });
  });

  // ── Realistic user mistakes ────────────────────────────────────────────

  describe("realistic user mistakes", () => {
    it("catches invalid enum values across trigger types", () => {
      // User typos an http_method as uppercase
      const httpResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/http_handler",
          is_flow: false,
          route_path: "api/webhook",
          request_type: "sync",
          authentication_method: "none",
          http_method: "POST",
          is_static_website: false,
          workspaced_route: false,
          wrap_body: false,
          raw_string: false,
        }),
        { type: "trigger", triggerKind: "http" }
      );
      expect(httpResult.errors.length).toBeGreaterThan(0);

      // User writes "iam_role" instead of "oidc" or "credentials"
      const sqsResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/sqs_handler",
          is_flow: false,
          queue_url: "https://sqs.us-east-1.amazonaws.com/12345/q",
          aws_resource_path: "f/resources/aws",
          aws_auth_resource_type: "iam_role",
        }),
        { type: "trigger", triggerKind: "sqs" }
      );
      expect(sqsResult.errors.length).toBeGreaterThan(0);

      // User writes "stream" instead of "push" or "pull"
      const gcpResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/gcp_handler",
          is_flow: false,
          gcp_resource_path: "f/resources/gcp",
          topic_id: "t",
          subscription_id: "s",
          delivery_type: "stream",
          subscription_mode: "existing",
        }),
        { type: "trigger", triggerKind: "gcp" }
      );
      expect(gcpResult.errors.length).toBeGreaterThan(0);

      // User writes "v4" instead of "v3" or "v5"
      const mqttResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/mqtt_handler",
          is_flow: false,
          mqtt_resource_path: "f/resources/mqtt",
          subscribe_topics: [{ topic: "t", qos: "qos1" }],
          client_version: "v4",
        }),
        { type: "trigger", triggerKind: "mqtt" }
      );
      expect(mqttResult.errors.length).toBeGreaterThan(0);
    });

    it("catches malformed nested structures", () => {
      // MQTT topic entry missing qos
      const mqttResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/mqtt_handler",
          is_flow: false,
          mqtt_resource_path: "f/resources/mqtt",
          subscribe_topics: [{ topic: "test/topic" }],
        }),
        { type: "trigger", triggerKind: "mqtt" }
      );
      expect(mqttResult.errors.length).toBeGreaterThan(0);

      // HTTP static_asset_config without required s3 field
      const httpResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/http_handler",
          is_flow: false,
          route_path: "api/assets",
          request_type: "sync",
          authentication_method: "none",
          http_method: "get",
          is_static_website: true,
          workspaced_route: false,
          wrap_body: false,
          raw_string: false,
          static_asset_config: { filename: "index.html" },
        }),
        { type: "trigger", triggerKind: "http" }
      );
      expect(httpResult.errors.length).toBeGreaterThan(0);

      // GCP delivery_config without required authenticate / base_endpoint
      const gcpResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/gcp_handler",
          is_flow: false,
          gcp_resource_path: "f/resources/gcp",
          topic_id: "t",
          subscription_id: "s",
          delivery_type: "push",
          subscription_mode: "create_update",
          delivery_config: { audience: "test" },
        }),
        { type: "trigger", triggerKind: "gcp" }
      );
      expect(gcpResult.errors.length).toBeGreaterThan(0);

      // Websocket filter missing key
      const wsResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/ws_handler",
          is_flow: false,
          url: "wss://example.com/socket",
          filters: [{ value: "test" }],
          can_return_message: false,
          can_return_error_result: true,
        }),
        { type: "trigger", triggerKind: "websocket" }
      );
      expect(wsResult.errors.length).toBeGreaterThan(0);
    });

    it("catches wrong types in YAML (string-for-boolean, object-for-array)", () => {
      // is_flow: "false" — YAML without quotes would parse to boolean,
      // but a quoted "false" parses as string
      const natsResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/nats_handler",
          is_flow: "false",
          nats_resource_path: "f/resources/nats",
          use_jetstream: "yes",
          subjects: ["events.>"],
        }),
        { type: "trigger", triggerKind: "nats" }
      );
      expect(natsResult.errors.length).toBeGreaterThanOrEqual(2);

      // topics as {} instead of []
      const kafkaResult = validator.validate(
        JSON.stringify({
          script_path: "f/triggers/kafka_handler",
          is_flow: false,
          kafka_resource_path: "f/resources/kafka",
          group_id: "g",
          topics: {},
          filters: "not-an-array",
        }),
        { type: "trigger", triggerKind: "kafka" }
      );
      expect(kafkaResult.errors.length).toBeGreaterThanOrEqual(2);
    });
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

  it("detects all 8 trigger kinds", () => {
    const kinds = [
      "http",
      "websocket",
      "kafka",
      "nats",
      "postgres",
      "mqtt",
      "sqs",
      "gcp",
    ] as const;

    for (const kind of kinds) {
      expect(
        getValidationTargetFromFilename(
          `f/triggers/handler.${kind}_trigger.yaml`
        )
      ).toEqual({
        type: "trigger",
        triggerKind: kind,
      });
    }
  });

  it("is case-insensitive for extensions", () => {
    expect(getValidationTargetFromFilename("f/my.flow/flow.YAML")).toEqual({
      type: "flow",
    });
    expect(
      getValidationTargetFromFilename("f/folder/daily.schedule.YML")
    ).toEqual({
      type: "schedule",
    });
  });

  it("handles dots in directory names", () => {
    expect(
      getValidationTargetFromFilename("f/my.app.flow/flow.yaml")
    ).toEqual({
      type: "flow",
    });
  });

  it("returns email trigger target for email_trigger files", () => {
    expect(
      getValidationTargetFromFilename("u/user/mail.email_trigger.yaml")
    ).toEqual({ type: "trigger", triggerKind: "email" });
  });

  it("returns null for unsupported trigger kinds and non-windmill files", () => {
    expect(getValidationTargetFromFilename("README.md")).toBeNull();
    expect(getValidationTargetFromFilename("f/folder/script.py")).toBeNull();
    expect(
      getValidationTargetFromFilename("f/folder/resource.yaml")
    ).toBeNull();
  });
});

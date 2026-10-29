/**
 * `wmill trigger new` scaffolds only fields the backend reads: `request_type`
 * rather than the deprecated `is_async`, no `requires_auth` (its column was
 * renamed to `authentication_method`), and `mode` rather than the deprecated
 * `enabled`.
 */
import { expect, test, describe } from "bun:test";
import { triggerTemplates } from "../src/commands/trigger/trigger.ts";

describe("trigger new templates", () => {
  test("the http template has no is_async or requires_auth", () => {
    const http = triggerTemplates.http;
    expect(Object.keys(http)).not.toContain("is_async");
    expect(Object.keys(http)).not.toContain("requires_auth");
    expect(http.request_type).toBe("sync");
    expect(http.authentication_method).toBe("none");
  });

  test("no template scaffolds the deprecated enabled field", () => {
    const withEnabled = Object.entries(triggerTemplates)
      .filter(([, template]) => "enabled" in template)
      .map(([kind]) => kind);
    expect(withEnabled).toEqual([]);
  });

  test("templates that scaffolded a disabled trigger still do, through mode", () => {
    const modes = Object.fromEntries(
      Object.entries(triggerTemplates).map(([kind, t]) => [kind, t.mode]),
    );
    expect(modes).toEqual({
      http: undefined,
      websocket: "disabled",
      kafka: "disabled",
      nats: "disabled",
      postgres: "disabled",
      mqtt: "disabled",
      amqp: "disabled",
      sqs: "disabled",
      gcp: "disabled",
      azure: "disabled",
      email: "disabled",
    });
  });
});

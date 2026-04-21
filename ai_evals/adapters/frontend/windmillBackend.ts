import { randomUUID } from "node:crypto";
import type { WindmillBackendSettings } from "../../core/windmillBackendSettings";

const tokenCache = new Map<string, Promise<string>>();
const sharedWorkspaceQueue = new Map<string, Promise<void>>();

export class WindmillBackendClient {
  constructor(private readonly settings: WindmillBackendSettings) {}

  async withWorkspace<T>(
    caseId: string,
    attempt: number,
    body: (workspaceId: string) => Promise<T>,
  ): Promise<T> {
    const workspaceId =
      this.settings.workspaceOverride ??
      buildWorkspaceId(this.settings.workspacePrefix, caseId, attempt);

    const run = async () => {
      await this.ensureWorkspace(workspaceId);

      try {
        return await body(workspaceId);
      } finally {
        if (!this.settings.keepWorkspaces && !this.settings.workspaceOverride) {
          await this.deleteWorkspace(workspaceId).catch(() => undefined);
        }
      }
    };

    if (this.settings.workspaceOverride) {
      return await withSharedWorkspaceLock(workspaceId, run);
    }

    return await run();
  }

  async request(path: string, init?: RequestInit): Promise<Response> {
    const token = await this.getToken();
    return await fetch(`${this.settings.baseUrl}/api${path}`, {
      ...init,
      headers: {
        Authorization: `Bearer ${token}`,
        ...(init?.headers ?? {}),
      },
    });
  }

  async getToken(): Promise<string> {
    const cacheKey = `${this.settings.baseUrl}|${this.settings.email}`;
    let tokenPromise = tokenCache.get(cacheKey);
    if (!tokenPromise) {
      tokenPromise = this.login().catch((error) => {
        if (tokenCache.get(cacheKey) === tokenPromise) {
          tokenCache.delete(cacheKey);
        }
        throw error;
      });
      tokenCache.set(cacheKey, tokenPromise);
    }
    return await tokenPromise;
  }

  async upsertResource(input: {
    workspaceId: string;
    path: string;
    resourceType: string;
    value: Record<string, unknown>;
  }): Promise<void> {
    const response = await this.request(
      `/w/${encodeURIComponent(input.workspaceId)}/resources/create?update_if_exists=true`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: input.path,
          resource_type: input.resourceType,
          value: input.value,
        }),
      },
    );
    await expectOk(response, `upsert resource ${input.path}`);
  }

  private async ensureWorkspace(workspaceId: string): Promise<void> {
    const existsResponse = await this.request("/workspaces/exists", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id: workspaceId }),
    });
    await expectOk(existsResponse, `check workspace ${workspaceId}`);

    if ((await existsResponse.text()).trim() === "true") {
      return;
    }

    const createResponse = await this.request("/workspaces/create", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id: workspaceId, name: workspaceId }),
    });
    try {
      await expectOk(createResponse, `create workspace ${workspaceId}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (message.includes("maximum number of workspaces")) {
        throw new Error(
          `${message}. Reuse an existing workspace with WMILL_AI_EVAL_BACKEND_WORKSPACE=<workspace-id>.`,
        );
      }
      throw error;
    }
  }

  private async deleteWorkspace(workspaceId: string): Promise<void> {
    const response = await this.request(
      `/workspaces/delete/${encodeURIComponent(workspaceId)}`,
      {
        method: "DELETE",
      },
    );
    await expectOk(response, `delete workspace ${workspaceId}`);
  }

  private async login(): Promise<string> {
    const response = await fetch(`${this.settings.baseUrl}/api/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: this.settings.email,
        password: this.settings.password,
      }),
    });
    await expectOk(response, "login to Windmill backend");
    return (await response.text()).trim();
  }
}

async function withSharedWorkspaceLock<T>(
  workspaceId: string,
  body: () => Promise<T>,
): Promise<T> {
  const previous = sharedWorkspaceQueue.get(workspaceId) ?? Promise.resolve();
  let releaseCurrent: (() => void) | undefined;
  const current = new Promise<void>((resolve) => {
    releaseCurrent = resolve;
  });
  const tail = previous.catch(() => undefined).then(() => current);
  sharedWorkspaceQueue.set(workspaceId, tail);

  await previous.catch(() => undefined);

  try {
    return await body();
  } finally {
    releaseCurrent?.();
    if (sharedWorkspaceQueue.get(workspaceId) === tail) {
      sharedWorkspaceQueue.delete(workspaceId);
    }
  }
}

function buildWorkspaceId(
  prefix: string,
  caseId: string,
  attempt: number,
): string {
  const caseSlug = caseId
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 30);
  const suffix = randomUUID().slice(0, 8);
  return `${prefix}-${caseSlug || "case"}-a${attempt}-${suffix}`;
}

async function expectOk(response: Response, context: string): Promise<void> {
  if (response.ok) {
    return;
  }
  throw new Error(
    `${context} failed: ${response.status} ${response.statusText} - ${await response.text()}`,
  );
}

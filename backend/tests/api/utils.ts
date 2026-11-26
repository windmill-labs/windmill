import { OpenAPI, createWorkspace, getJob, login } from "../../../cli/gen/index.ts";

// Setup method for each api test
export async function setup() {
    await generateApiToken()
    const workspace = await genTestWorkspace();

    return { workspace }
}

async function generateApiToken() {
    // if WM_TOKEN env variable not set
    if (!OpenAPI.TOKEN) {
        const email = Deno.env.get("LOGIN_WM_EMAIL") || "admin@windmill.dev";
        const password = Deno.env.get("LOGIN_WM_PASSWORD") || "changeme";
        const token = await login({ requestBody: { email, password } });
        OpenAPI.TOKEN = token;
    }

}

/**
 * Ensure a workspace named "api-tests" is created
 */
async function genTestWorkspace() {
    const id = "api-tests"

    await createWorkspace({ requestBody: { id, name: id}}).catch(()=> null)

    return id
}

/**
 * 
 * @returns The job result
 */
export async function awaitJobCompletion(data: { workspace: string, id: string, timeoutMs: number }) {
    const start = Date.now();
    while (true) {
        const result = await getJob(data).catch(()=> null);


        if (!!result && result.type === "CompletedJob") {
            return result;
        }

        if ((Date.now() - start) > data.timeoutMs) {
            throw new Error("Job did not finish before the specified timeout: " + data.timeoutMs)
        }
    }
}
import { ResourceService, VariableService } from './windmill-api/index.ts'
import { OpenAPI } from './windmill-api/index.ts'

export {
    AdminService, AuditService, FlowService, GranularAclService, GroupService,
    JobService, ResourceService, VariableService, ScriptService, ScheduleService, SettingsService,
    UserService, WorkspaceService
} from './windmill-api/index.ts'

export { pgSql, pgClient } from './pg.ts'

export type Sql = string
export type Email = string
export type Base64 = string
export type Resource<S extends string> = any

export const SHARED_FOLDER = '/shared'

export function setClient(token: string, baseUrl: string) {
    OpenAPI.WITH_CREDENTIALS = true
    OpenAPI.TOKEN = token
    OpenAPI.BASE = baseUrl + '/api'
}

setClient(Deno.env.get("WM_TOKEN") ?? 'no_token', Deno.env.get("BASE_INTERNAL_URL") ?? Deno.env.get("BASE_URL") ?? 'http://localhost:8000')

/**
 * Create a client configuration from env variables
 * @returns client configuration
 */
export function getWorkspace(): string {
    return Deno.env.get("WM_WORKSPACE") ?? 'no_workspace'
}

/**
 * Get a resource value by path
 * @param path path of the resource
 * @param undefinedIfEmpty if the resource does not exist, return undefined instead of throwing an error
 * @returns resource value
 */
export async function getResource(path: string, undefinedIfEmpty?: boolean): Promise<any> {
    const workspace = getWorkspace()
    try {
        const resource = await ResourceService.getResource({ workspace, path })
        return await _transformLeaf(resource.value)
    } catch (e: any) {
        if (undefinedIfEmpty && e.status === 404) {
            return undefined
        } else {
            throw e
        }
    }
}

export function getInternalStatePath(suffix?: string): string {
    const env_flow_path = Deno.env.get("WM_FLOW_PATH")
    const env_job_path = Deno.env.get("WM_JOB_PATH")
    const permissioned_as = Deno.env.get("WM_PERMISSIONED_AS")
    const flow_path = env_flow_path != undefined && env_flow_path != "" ? env_flow_path : 'NO_FLOW_PATH'
    const script_path = suffix ?? (env_job_path != undefined && env_job_path != "" ? env_job_path : 'NO_JOB_PATH')
    const env_schedule_path = Deno.env.get("WM_SCHEDULE_PATH")
    const schedule_path = env_schedule_path != undefined && env_schedule_path != "" ? `/${env_schedule_path}` : ''

    if (script_path.slice(script_path.length - 1) === '/') {
        throw Error(`The script path must not end with '/', give a name to your script!`)
    }
    return `${permissioned_as}/${flow_path}/${script_path}${schedule_path}`
}

/**
 * Set a resource value by path
 * @param path path of the resource to set
 * @param value new value of the resource to set
 * @param initializeToTypeIfNotExist if the resource does not exist, initialize it with this type
 */
export async function setResource(path: string, value: any, initializeToTypeIfNotExist?: string): Promise<void> {
    const workspace = getWorkspace()
    if (await ResourceService.existsResource({ workspace, path })) {
        await ResourceService.updateResource({ workspace, path, requestBody: { value } })
    } else if (initializeToTypeIfNotExist) {
        await ResourceService.createResource({ workspace, requestBody: { path, value, resource_type: initializeToTypeIfNotExist } })
    } else {
        throw Error(`Resource at path ${path} does not exist and no type was provided to initialize it`)
    }
}

/**
 * Set the internal state
 * @param state state to set
 * @param suffix suffix of the path of the internal state (useful to share internal state between jobs)
 */
export async function setInternalState(state: any, suffix?: string): Promise<void> {
    await setResource(getInternalStatePath(suffix), state, 'state')
}

/**
 * Get the internal state
 * @param suffix suffix of the path of the internal state (useful to share internal state between jobs)
 */
export async function getInternalState(suffix?: string): Promise<any> {
    return await getResource(getInternalStatePath(suffix), true)
}

/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
export async function getVariable(path: string): Promise<string | undefined> {
    const workspace = getWorkspace()
    const variable = await VariableService.getVariable({ workspace, path })
    return variable.value
}

async function transformLeaves(d: { [key: string]: any }): Promise<{ [key: string]: any }> {
    for (const k in d) {
        d[k] = await _transformLeaf(d[k])
    }
    return d
}

const VAR_RESOURCE_PREFIX = "$var:"
async function _transformLeaf(v: any): Promise<any> {
    if (typeof v === 'object') {
        return transformLeaves(v)
    }
    else if (typeof v === 'string' && v.startsWith(VAR_RESOURCE_PREFIX)) {
        const varName = v.substring(VAR_RESOURCE_PREFIX.length)
        return await getVariable(varName)
    } else {
        return v
    }
}

export async function databaseUrlFromResource(path: string): Promise<string> {
    const resource = await getResource(path)
    return `postgresql://${resource.user}:${resource.password}@${resource.host}:${resource.port}/${resource.dbname}?sslmode=${resource.sslmode}`
}


export interface NonceAndHmac {
    nonce: number;
    signature: string;
}

/**
 * Get HMAC and nonce needed for approval script
 * @param workspace workspace name
 * @param jobId
 * @param approver approver name
 * @returns HMAC and nonce needed to authorize approval script actions
 */
export async function genNounceAndHmac(workspace: string, jobId: string, approver?: string): Promise<NonceAndHmac> {
    const nonce = Math.floor(Math.random() * 4294967295);
    const sig = await fetch(Deno.env.get("WM_BASE_URL") +
        `/api/w/${workspace}/jobs/job_signature/${jobId}/${nonce}?token=${Deno.env.get("WM_TOKEN")}${approver ? `&approver=${approver}` : ''}`)
    return {
        nonce,
        signature: await sig.text()
    };
}

export interface ResumeEndpoints {
    approvalPage: string;
    resume: string;
    cancel: string;
}

/**
 * Get URLs needed for approval script
 * @param approver approver name
 * @returns approval page UI URL, resume and cancel API URLs for approval script
 */
export async function getResumeEndpoints(approver?: string): Promise<ResumeEndpoints> {
    const workspace = getWorkspace()

    const { nonce, signature } = await genNounceAndHmac(
        workspace,
        Deno.env.get("WM_JOB_ID") ?? "no_job_id",
        approver
    );

    function getResumeUrl(op: string): string {
        const u = new URL(
            `${op}/${Deno.env.get("WM_JOB_ID")}/${nonce}/${signature}`,
            Deno.env.get("WM_BASE_URL") + `/api/w/${workspace}/jobs/`,
        );
        if (approver) {
            u.searchParams.append('approver', approver);
        }

        return u.toString();
    }

    function getApprovalPage() {
        const u = new URL(
            `/approve/${workspace}/${Deno.env.get("WM_JOB_ID")}/${nonce}/${signature}`,
            Deno.env.get("WM_BASE_URL"),
        );
        if (approver) {
            u.searchParams.append('approver', approver);
        }

        return u.toString();
    }

    return {
        approvalPage: getApprovalPage(),
        resume: getResumeUrl("resume"),
        cancel: getResumeUrl("cancel"),
    };
}

export function base64ToUint8Array(data: string): Uint8Array {
    return Uint8Array.from(atob(data), c => c.charCodeAt(0))
}

export function uint8ArrayToBase64(arrayBuffer: Uint8Array): string {
    let base64 = ''
    const encodings = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'

    const bytes = new Uint8Array(arrayBuffer)
    const byteLength = bytes.byteLength
    const byteRemainder = byteLength % 3
    const mainLength = byteLength - byteRemainder

    let a, b, c, d
    let chunk

    // Main loop deals with bytes in chunks of 3
    for (let i = 0; i < mainLength; i = i + 3) {
        // Combine the three bytes into a single integer
        chunk = (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2]

        // Use bitmasks to extract 6-bit segments from the triplet
        a = (chunk & 16515072) >> 18 // 16515072 = (2^6 - 1) << 18
        b = (chunk & 258048) >> 12 // 258048   = (2^6 - 1) << 12
        c = (chunk & 4032) >> 6 // 4032     = (2^6 - 1) << 6
        d = chunk & 63               // 63       = 2^6 - 1

        // Convert the raw binary segments to the appropriate ASCII encoding
        base64 += encodings[a] + encodings[b] + encodings[c] + encodings[d]
    }

    // Deal with the remaining bytes and padding
    if (byteRemainder == 1) {
        chunk = bytes[mainLength]

        a = (chunk & 252) >> 2 // 252 = (2^6 - 1) << 2

        // Set the 4 least significant bits to zero
        b = (chunk & 3) << 4 // 3   = 2^2 - 1

        base64 += encodings[a] + encodings[b] + '=='
    } else if (byteRemainder == 2) {
        chunk = (bytes[mainLength] << 8) | bytes[mainLength + 1]

        a = (chunk & 64512) >> 10 // 64512 = (2^6 - 1) << 10
        b = (chunk & 1008) >> 4 // 1008  = (2^6 - 1) << 4

        // Set the 2 least significant bits to zero
        c = (chunk & 15) << 2 // 15    = 2^4 - 1

        base64 += encodings[a] + encodings[b] + encodings[c] + '='
    }

    return base64
}

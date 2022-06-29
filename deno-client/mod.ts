import { ResourceApi, VariableApi, ServerConfiguration } from './windmill-api/index.ts'
import { createConfiguration, type Configuration as Configuration } from './windmill-api/configuration.ts'

export {
    AdminApi, AuditApi, FlowApi, GranularAclApi, GroupApi,
    JobApi, ResourceApi, VariableApi, ScriptApi, ScheduleApi, SettingsApi,
    UserApi, WorkspaceApi
} from './windmill-api/index.ts'

export type Email = string
export type Base64 = string
export type Resource<S extends string> = {}

/**
 * Create a client configuration from env variables
 * @returns client configuration
 */
export function createConf(): Configuration & { workspace_id: string } {
    const token = Deno.env.get("WM_TOKEN") ?? 'no_token'
    const base_url = Deno.env.get("BASE_INTERNAL_URL") ?? 'http://localhost:8000'
    return {
        ...createConfiguration({
            baseServer: new ServerConfiguration(`${base_url}/api`, {}),
            authMethods: { bearerAuth: { tokenProvider: { getToken() { return token } } } },
        }), workspace_id: Deno.env.get("WM_WORKSPACE") ?? 'no_workspace'
    }
}

/**
 * Get a resource value by path
 * @param path path of the resource
 * @returns resource value
 */
export async function getResource(path: string, initializeToTypeIfNotExist?: string): Promise<any> {
    const conf = createConf()
    try {
        const resource = await new ResourceApi(conf).getResource(conf.workspace_id, path)
        return await transformLeaves(resource.value)
    } catch (e) {
        if (initializeToTypeIfNotExist && e.code === 404) {
            await new ResourceApi(conf).createResource(conf.workspace_id, { path, value: {}, resourceType: initializeToTypeIfNotExist })
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

    return `${permissioned_as}/${flow_path}/${script_path}`
}

/**
 * Set a resource value by path
 * @param path path of the resource to set
 * @param value new value of the resource to set
 * @param initializeToTypeIfNotExist if the resource does not exist, initialize it with this type
 */
export async function setResource(path: string, value: any, initializeToTypeIfNotExist?: string): Promise<void> {
    const conf = createConf()
    try {
        await new ResourceApi(conf).updateResource(conf.workspace_id, path, { value })
    } catch (e) {
        if (initializeToTypeIfNotExist && e.code === 404) {
            await new ResourceApi(conf).createResource(conf.workspace_id, { path, value, resourceType: initializeToTypeIfNotExist })
        } else {
            throw e
        }
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
    return await getResource(getInternalStatePath(suffix), 'state')
}

/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
export async function getVariable(path: string): Promise<string | undefined> {
    const conf = createConf()
    const variable = await new VariableApi(conf).getVariable(conf.workspace_id, path)
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
    return `postgresql://${resource.user}:${resource.password}@${resource.host}:${resource.port}/${resource.database}?sslmode=${resource.sslmode}`
}

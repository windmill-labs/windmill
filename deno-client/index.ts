import { ResourceApi, VariableApi, ServerConfiguration } from './windmill-api/index.ts'
import { createConfiguration, type Configuration as Configuration } from './windmill-api/configuration.ts'

export {
    AdminApi, AuditApi, FlowApi, GranularAclApi, GroupApi,
    JobApi, ResourceApi, VariableApi, ScriptApi, ScheduleApi, SettingsApi,
    UserApi, WorkspaceApi
} from './windmill-api/index.ts'

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
export async function getResource(path: string): Promise<any> {
    const conf = createConf()
    const resource = await new ResourceApi(conf).getResource(conf.workspace_id, path)
    return await transformLeaves(resource.value)
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

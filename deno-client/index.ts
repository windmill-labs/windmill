import { ResourceApi, VariableApi, ServerConfiguration } from './windmill-api/index.ts'
import { createConfiguration, type Configuration as Configuration } from './windmill-api/configuration.ts'

export * from './windmill-api/index.ts'

export function createConf(): Configuration & { workspace_id: string } {
    const token = Deno.env.get("WM_TOKEN") ?? 'no_token'
    return {
        ...createConfiguration({
            baseServer: new ServerConfiguration(Deno.env.get("BASE_INTERNAL_URL") ?? 'no_base_url', {}),
            authMethods: { bearerAuth: { tokenProvider: { getToken() { return token } } } },
        }), workspace_id: Deno.env.get("WM_WORKSPACE") ?? 'no_workspace'
    }
}

export async function getResource(path: string): Promise<any> {
    const conf = createConf()
    const resource = await new ResourceApi(conf).getResource(path, conf.workspace_id)
    return await transformLeaves(resource.value)
}

export async function getVariable(path: string): Promise<string | undefined> {
    const conf = createConf()
    const variable = await new VariableApi(conf).getVariable(path, conf.workspace_id)
    return variable.value
}

export async function transformLeaves(d: { [key: string]: any }): Promise<{ [key: string]: any }> {
    for (const k in d) {
        d[k] = await _transformLeaf(d[k])
    }
    return d
}

const VAR_RESOURCE_PREFIX = "$VAR:"
async function _transformLeaf(v: any): Promise<any> {
    if (typeof v === 'object') {
        return transformLeaves(v)
    }
    else if (typeof v === 'string' && v.startsWith(VAR_RESOURCE_PREFIX)) {
        const varName = v.substring(0, VAR_RESOURCE_PREFIX.length)
        return await getVariable(varName)
    } else {
        return v
    }
}

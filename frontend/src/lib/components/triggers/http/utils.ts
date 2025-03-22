import { base } from "$lib/base";
import { isCloudHosted } from "$lib/cloud";

export function getHttpRoute(route_path: string | undefined, workspaced_route: boolean, workspace_id: string) {
    return `${location.origin}${base}/api/r/${
        isCloudHosted() || workspaced_route ? workspace_id + '/' : ''
    }${route_path ?? ''}`
}

export const HTTP_TRIGGERS_QUERY_PARAM_NAMES = 'http_trigger_variables'
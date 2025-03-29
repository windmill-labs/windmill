import { base } from "$lib/base";
import { isCloudHosted } from "$lib/cloud";

export function getHttpRoute(route_path: string | undefined, workspaced_route: boolean, workspace_id: string, route_prefix: string) {
    return `${location.origin}${base}/api/${route_prefix.replace(/^\/|\/$/g, '')}/${
        isCloudHosted() || workspaced_route ? workspace_id + '/' : ''
    }${route_path ?? ''}`
}
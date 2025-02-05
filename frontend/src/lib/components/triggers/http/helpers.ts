import { base } from '$lib/base'

import { isCloudHosted } from '$lib/cloud'

export function getHttpRoute(route_path: string | undefined, workspace: string) {
	return `${location.origin}${base}/api/r/${isCloudHosted() ? workspace + '/' : ''}${
		route_path ?? ''
	}`
}

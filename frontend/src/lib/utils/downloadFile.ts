import { OpenAPI } from '$lib/gen'
import { getHeaders } from '$lib/gen/core/request'
import { sendUserToast } from '$lib/toast'

/**
 * When OpenAPI is configured to authenticate via headers (TOKEN, basic auth,
 * or arbitrary HEADERS) we cannot rely on a plain `<a href>` browser
 * navigation because the browser does not attach those headers. In that case
 * fetch the file via the OpenAPI client (which carries the configured auth)
 * and trigger a download from a blob URL. Cookie-only auth still works with
 * a plain link.
 *
 * `apiPath` should be the path relative to OpenAPI.BASE, starting with `/`
 * (e.g. `/w/foo/job_helpers/download_s3_file?file_key=...`).
 */
export function shouldDownloadViaClient(): boolean {
	return Boolean(OpenAPI.TOKEN || OpenAPI.HEADERS || OpenAPI.USERNAME)
}

export async function downloadViaClient(apiPath: string, filename: string): Promise<void> {
	const url = `${OpenAPI.BASE}${apiPath}`
	let response: Response
	try {
		const headers = await getHeaders(OpenAPI, { method: 'GET', url: apiPath })
		response = await fetch(url, { headers, credentials: OpenAPI.CREDENTIALS })
	} catch (e) {
		sendUserToast(`Download failed: ${e}`, true)
		return
	}
	if (!response.ok) {
		sendUserToast(`Download failed: ${response.status} ${response.statusText}`, true)
		return
	}
	const blob = await response.blob()
	const blobUrl = URL.createObjectURL(blob)
	const a = document.createElement('a')
	a.href = blobUrl
	a.download = filename
	document.body.appendChild(a)
	a.click()
	a.remove()
	URL.revokeObjectURL(blobUrl)
}

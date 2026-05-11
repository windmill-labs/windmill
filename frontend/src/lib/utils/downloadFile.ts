import { OpenAPI } from '$lib/gen'
import { sendUserToast } from '$lib/toast'

async function resolveToken(): Promise<string | undefined> {
	const t = OpenAPI.TOKEN
	if (!t) return undefined
	return typeof t === 'string' ? t : await t({} as any)
}

/**
 * When OpenAPI.TOKEN is set we cannot rely on a plain `<a href>` browser navigation
 * because the browser does not attach the Authorization header. In that case fetch
 * the file via the OpenAPI client (which uses OpenAPI.BASE and the Bearer token) and
 * trigger a download from a blob URL. Otherwise let the default link behavior happen.
 *
 * `apiPath` should be the path relative to OpenAPI.BASE, starting with `/`
 * (e.g. `/w/foo/job_helpers/download_s3_file?file_key=...`).
 */
export function shouldDownloadViaClient(): boolean {
	return Boolean(OpenAPI.TOKEN)
}

export async function downloadViaClient(apiPath: string, filename: string): Promise<void> {
	const token = await resolveToken()
	const url = `${OpenAPI.BASE}${apiPath}`
	const headers: Record<string, string> = {}
	if (token) headers['Authorization'] = `Bearer ${token}`
	let response: Response
	try {
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

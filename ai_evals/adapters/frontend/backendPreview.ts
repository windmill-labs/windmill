import { randomUUID } from 'node:crypto'
import type { BackendValidationSettings } from '../../core/backendValidation'

interface CompletedJobResultMaybe {
	completed: boolean
	result: unknown
	success?: boolean
	started?: boolean
}

export interface CompletedPreviewJob {
	id: string
	success: boolean
	result: unknown
	logs?: string | null
	raw: Record<string, unknown>
}

const tokenCache = new Map<string, Promise<string>>()

export class BackendPreviewClient {
	constructor(private readonly settings: BackendValidationSettings) {}

	async withWorkspace<T>(
		caseId: string,
		attempt: number,
		body: (workspaceId: string) => Promise<T>
	): Promise<T> {
		const workspaceId =
			this.settings.workspaceOverride ??
			buildWorkspaceId(this.settings.workspacePrefix, caseId, attempt)
		await this.ensureWorkspace(workspaceId)

		try {
			return await body(workspaceId)
		} finally {
			if (!this.settings.keepWorkspaces && !this.settings.workspaceOverride) {
				await this.deleteWorkspace(workspaceId).catch(() => undefined)
			}
		}
	}

	async createScript(input: {
		workspaceId: string
		path: string
		summary: string
		description?: string
		schema?: Record<string, unknown>
		content: string
		language: string
	}): Promise<void> {
		await this.ensureFolderForPath(input.workspaceId, input.path)

		const response = await this.request(`/w/${encodeURIComponent(input.workspaceId)}/scripts/create`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				path: input.path,
				summary: input.summary,
				description: input.description ?? '',
				content: input.content,
				schema: input.schema ?? { type: 'object', properties: {}, required: [] },
				is_template: false,
				language: input.language,
				kind: 'script'
			})
		})
		await expectOkOrAlreadyExists(response, `create script ${input.path}`)
	}

	async createFlow(input: {
		workspaceId: string
		path: string
		summary: string
		description?: string
		schema?: Record<string, unknown>
		value: Record<string, unknown>
	}): Promise<void> {
		await this.ensureFolderForPath(input.workspaceId, input.path)

		const response = await this.request(`/w/${encodeURIComponent(input.workspaceId)}/flows/create`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				path: input.path,
				summary: input.summary,
				description: input.description ?? '',
				schema: input.schema ?? { type: 'object', properties: {}, required: [] },
				value: input.value
			})
		})
		await expectOkOrAlreadyExists(response, `create flow ${input.path}`)
	}

	async runScriptPreview(input: {
		workspaceId: string
		content: string
		args: Record<string, unknown>
		language: string
		path?: string
		timeoutSeconds?: number
	}): Promise<CompletedPreviewJob> {
		const response = await this.request(
			withQuery(`/w/${encodeURIComponent(input.workspaceId)}/jobs/run/preview`, {
				timeout: input.timeoutSeconds
			}),
			{
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					content: input.content,
					args: input.args,
					language: input.language,
					path: input.path
				})
			}
		)

		await expectOk(response, 'start script preview')
		const jobId = (await response.text()).trim()
		return await this.waitForCompletedJob(input.workspaceId, jobId)
	}

	async runFlowPreview(input: {
		workspaceId: string
		value: Record<string, unknown>
		args: Record<string, unknown>
		timeoutSeconds?: number
		path?: string
	}): Promise<CompletedPreviewJob> {
		const response = await this.request(
			withQuery(`/w/${encodeURIComponent(input.workspaceId)}/jobs/run/preview_flow`, {
				timeout: input.timeoutSeconds
			}),
			{
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					value: input.value,
					args: input.args,
					path: input.path
				})
			}
		)

		await expectOk(response, 'start flow preview')
		const jobId = (await response.text()).trim()
		return await this.waitForCompletedJob(input.workspaceId, jobId)
	}

	private async ensureWorkspace(workspaceId: string): Promise<void> {
		const existsResponse = await this.request('/workspaces/exists', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ id: workspaceId })
		})
		await expectOk(existsResponse, `check workspace ${workspaceId}`)

		if ((await existsResponse.text()).trim() === 'true') {
			return
		}

		const createResponse = await this.request('/workspaces/create', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ id: workspaceId, name: workspaceId })
		})
		try {
			await expectOk(createResponse, `create workspace ${workspaceId}`)
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error)
			if (message.includes('maximum number of workspaces')) {
				throw new Error(
					`${message}. Reuse an existing workspace with WMILL_AI_EVAL_BACKEND_WORKSPACE=<workspace-id>.`
				)
			}
			throw error
		}
	}

	private async deleteWorkspace(workspaceId: string): Promise<void> {
		const response = await this.request(`/workspaces/delete/${encodeURIComponent(workspaceId)}`, {
			method: 'DELETE'
		})
		await expectOk(response, `delete workspace ${workspaceId}`)
	}

	private async ensureFolderForPath(workspaceId: string, path: string): Promise<void> {
		const folderName = extractFolderName(path)
		if (!folderName) {
			return
		}

		const response = await this.request(`/w/${encodeURIComponent(workspaceId)}/folders/create`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ name: folderName })
		})

		if (response.ok) {
			return
		}

		const message = await response.text()
		if (!message.toLowerCase().includes('already exists')) {
			throw new Error(`Failed to create folder ${folderName}: ${message}`)
		}
	}

	private async waitForCompletedJob(
		workspaceId: string,
		jobId: string
	): Promise<CompletedPreviewJob> {
		const deadline = Date.now() + this.settings.maxWaitMs

		while (Date.now() < deadline) {
			const maybeResponse = await this.request(
				`/w/${encodeURIComponent(workspaceId)}/jobs_u/completed/get_result_maybe/${encodeURIComponent(jobId)}?get_started=false`
			)
			await expectOk(maybeResponse, `poll job ${jobId}`)
			const maybeResult = (await maybeResponse.json()) as CompletedJobResultMaybe

			if (maybeResult.completed) {
				const completedResponse = await this.request(
					`/w/${encodeURIComponent(workspaceId)}/jobs_u/completed/get/${encodeURIComponent(jobId)}`
				)
				await expectOk(completedResponse, `get completed job ${jobId}`)
				const completedJob = (await completedResponse.json()) as Record<string, unknown>
				return {
					id: jobId,
					success: Boolean(maybeResult.success),
					result: maybeResult.result,
					logs:
						typeof completedJob.logs === 'string' || completedJob.logs === null
							? (completedJob.logs as string | null)
							: null,
					raw: completedJob
				}
			}

			await new Promise((resolve) => setTimeout(resolve, this.settings.pollIntervalMs))
		}

		throw new Error(`Timed out waiting for preview job ${jobId} to complete`)
	}

	private async request(path: string, init?: RequestInit): Promise<Response> {
		const token = await this.getToken()
		return await fetch(`${this.settings.baseUrl}/api${path}`, {
			...init,
			headers: {
				Authorization: `Bearer ${token}`,
				...(init?.headers ?? {})
			}
		})
	}

	private async getToken(): Promise<string> {
		const cacheKey = `${this.settings.baseUrl}|${this.settings.email}`
		let tokenPromise = tokenCache.get(cacheKey)
		if (!tokenPromise) {
			tokenPromise = this.login()
			tokenCache.set(cacheKey, tokenPromise)
		}
		return await tokenPromise
	}

	private async login(): Promise<string> {
		const response = await fetch(`${this.settings.baseUrl}/api/auth/login`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				email: this.settings.email,
				password: this.settings.password
			})
		})
		await expectOk(response, 'login for backend validation')
		return (await response.text()).trim()
	}
}

function buildWorkspaceId(prefix: string, caseId: string, attempt: number): string {
	const caseSlug = caseId
		.toLowerCase()
		.replace(/[^a-z0-9-]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 30)
	const suffix = randomUUID().slice(0, 8)
	return `${prefix}-${caseSlug || 'case'}-a${attempt}-${suffix}`
}

function extractFolderName(path: string): string | null {
	if (!path.startsWith('f/')) {
		return null
	}
	const segments = path.split('/').slice(1, -1)
	return segments.length > 0 ? segments.join('/') : null
}

function withQuery(
	path: string,
	params: Record<string, string | number | undefined>
): string {
	const query = new URLSearchParams()
	for (const [key, value] of Object.entries(params)) {
		if (value === undefined) {
			continue
		}
		query.set(key, String(value))
	}
	const suffix = query.toString()
	return suffix ? `${path}?${suffix}` : path
}

async function expectOk(response: Response, context: string): Promise<void> {
	if (response.ok) {
		return
	}
	throw new Error(`${context} failed: ${response.status} ${response.statusText} - ${await response.text()}`)
}

async function expectOkOrAlreadyExists(response: Response, context: string): Promise<void> {
	if (response.ok) {
		return
	}
	const message = await response.text()
	if (message.toLowerCase().includes('already exists')) {
		return
	}
	throw new Error(`${context} failed: ${response.status} ${response.statusText} - ${message}`)
}

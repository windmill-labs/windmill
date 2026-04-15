import { randomUUID } from 'node:crypto'
import type { BackendValidationSettings } from '../../core/backendValidation'

interface CompletedJobResultMaybe {
	completed: boolean
	result: unknown
	success?: boolean
	started?: boolean
}

interface ScriptDeploymentStatus {
	lock?: unknown
	lock_error_logs?: string | null
}

export interface CompletedPreviewJob {
	id: string
	success: boolean
	result: unknown
	logs?: string | null
	raw: Record<string, unknown>
}

const tokenCache = new Map<string, Promise<string>>()
const sharedWorkspaceQueue = new Map<string, Promise<void>>()
const managedSharedWorkspacePrefixes = ['f/evals/']

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

		const run = async () => {
			await this.ensureWorkspace(workspaceId)
			if (this.settings.workspaceOverride) {
				await this.clearManagedSharedWorkspaceAssets(workspaceId)
			}

			try {
				return await body(workspaceId)
			} finally {
				if (!this.settings.keepWorkspaces && !this.settings.workspaceOverride) {
					await this.deleteWorkspace(workspaceId).catch(() => undefined)
				}
			}
		}

		if (this.settings.workspaceOverride) {
			return await withSharedWorkspaceLock(workspaceId, run)
		}

		return await run()
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

		const payload = {
			path: input.path,
			summary: input.summary,
			description: input.description ?? '',
			content: input.content,
			schema: input.schema ?? { type: 'object', properties: {}, required: [] },
			is_template: false,
			language: input.language,
			kind: 'script'
		}

		const response = await this.request(`/w/${encodeURIComponent(input.workspaceId)}/scripts/create`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(payload)
		})

		if (response.ok) {
			await this.waitForScriptDeployment(input.workspaceId, input.path, (await response.text()).trim())
			return
		}

		const message = await response.text()
		if (!isConflictMessage(message)) {
			throw new Error(`create script ${input.path} failed: ${response.status} ${response.statusText} - ${message}`)
		}

		const currentScript = await this.getScriptByPath(input.workspaceId, input.path)
		const currentHash = readStringField(currentScript, 'hash', `script ${input.path}`)
		const updateResponse = await this.request(
			`/w/${encodeURIComponent(input.workspaceId)}/scripts/create`,
			{
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					...payload,
					parent_hash: currentHash
				})
			}
		)
		await expectOk(updateResponse, `update script ${input.path}`)
		await this.waitForScriptDeployment(input.workspaceId, input.path, (await updateResponse.text()).trim())
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

		const payload = {
			path: input.path,
			summary: input.summary,
			description: input.description ?? '',
			schema: input.schema ?? { type: 'object', properties: {}, required: [] },
			value: input.value
		}

		const response = await this.request(`/w/${encodeURIComponent(input.workspaceId)}/flows/create`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(payload)
		})

		if (response.ok) {
			return
		}

		const message = await response.text()
		if (!isConflictMessage(message)) {
			throw new Error(`create flow ${input.path} failed: ${response.status} ${response.statusText} - ${message}`)
		}

		const updateResponse = await this.request(
			`/w/${encodeURIComponent(input.workspaceId)}/flows/update/${input.path}`,
			{
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(payload)
			}
		)
		await expectOk(updateResponse, `update flow ${input.path}`)
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

	private async getScriptByPath(workspaceId: string, path: string): Promise<Record<string, unknown>> {
		const response = await this.request(`/w/${encodeURIComponent(workspaceId)}/scripts/get/p/${path}`)
		await expectOk(response, `get script ${path}`)
		return (await response.json()) as Record<string, unknown>
	}

	private async clearManagedSharedWorkspaceAssets(workspaceId: string): Promise<void> {
		const flowPaths = await this.listFlowPaths(workspaceId)
		for (const path of flowPaths.filter(isManagedSharedWorkspacePath)) {
			await this.deleteFlowByPath(workspaceId, path)
		}

		const scriptPaths = await this.listScriptPaths(workspaceId)
		for (const path of scriptPaths.filter(isManagedSharedWorkspacePath)) {
			await this.deleteScriptByPath(workspaceId, path)
		}
	}

	private async listFlowPaths(workspaceId: string): Promise<string[]> {
		const response = await this.request(`/w/${encodeURIComponent(workspaceId)}/flows/list_paths`)
		await expectOk(response, `list flows in workspace ${workspaceId}`)
		return await response.json()
	}

	private async listScriptPaths(workspaceId: string): Promise<string[]> {
		const response = await this.request(`/w/${encodeURIComponent(workspaceId)}/scripts/list_paths`)
		await expectOk(response, `list scripts in workspace ${workspaceId}`)
		return await response.json()
	}

	private async deleteFlowByPath(workspaceId: string, path: string): Promise<void> {
		const response = await this.request(`/w/${encodeURIComponent(workspaceId)}/flows/delete/${path}`, {
			method: 'DELETE'
		})
		await expectOk(response, `delete flow ${path}`)
	}

	private async deleteScriptByPath(workspaceId: string, path: string): Promise<void> {
		const response = await this.request(`/w/${encodeURIComponent(workspaceId)}/scripts/delete/p/${path}`, {
			method: 'POST'
		})
		await expectOk(response, `delete script ${path}`)
	}

	private async waitForScriptDeployment(
		workspaceId: string,
		path: string,
		hash: string
	): Promise<void> {
		const deadline = Date.now() + this.settings.maxWaitMs

		while (Date.now() < deadline) {
			const response = await this.request(
				`/w/${encodeURIComponent(workspaceId)}/scripts/deployment_status/h/${encodeURIComponent(hash)}`
			)
			await expectOk(response, `check deployment status for script ${path}`)
			const deployment = (await response.json()) as ScriptDeploymentStatus
			if (deployment.lock != null) {
				return
			}
			if (deployment.lock_error_logs) {
				throw new Error(`Script deployment failed for ${path}: ${deployment.lock_error_logs}`)
			}
			await new Promise((resolve) => setTimeout(resolve, this.settings.pollIntervalMs))
		}

		throw new Error(`Timed out waiting for script ${path} (${hash}) to deploy`)
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
			tokenPromise = this.login().catch((error) => {
				if (tokenCache.get(cacheKey) === tokenPromise) {
					tokenCache.delete(cacheKey)
				}
				throw error
			})
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

async function withSharedWorkspaceLock<T>(workspaceId: string, body: () => Promise<T>): Promise<T> {
	const previous = sharedWorkspaceQueue.get(workspaceId) ?? Promise.resolve()
	let releaseCurrent: (() => void) | undefined
	const current = new Promise<void>((resolve) => {
		releaseCurrent = resolve
	})
	const tail = previous.catch(() => undefined).then(() => current)
	sharedWorkspaceQueue.set(workspaceId, tail)

	await previous.catch(() => undefined)

	try {
		return await body()
	} finally {
		releaseCurrent?.()
		if (sharedWorkspaceQueue.get(workspaceId) === tail) {
			sharedWorkspaceQueue.delete(workspaceId)
		}
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

function readStringField(
	value: Record<string, unknown>,
	field: string,
	context: string
): string {
	const candidate = value[field]
	if (typeof candidate === 'string' && candidate.length > 0) {
		return candidate
	}
	throw new Error(`${context} is missing string field ${field}`)
}

function isConflictMessage(message: string): boolean {
	const normalized = message.toLowerCase()
	return normalized.includes('already exists') || normalized.includes('path conflict')
}

function isManagedSharedWorkspacePath(path: string): boolean {
	return managedSharedWorkspacePrefixes.some((prefix) => path.startsWith(prefix))
}

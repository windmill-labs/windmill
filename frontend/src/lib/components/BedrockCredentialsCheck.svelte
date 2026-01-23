<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { tryEvery } from '$lib/utils'
	import { Check, LoaderCircle, Server, X, Cpu } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'

	interface CredentialsCheckResult {
		available: boolean
		access_key_id_prefix?: string | null
		region?: string | null
		has_session_token: boolean
		source?: string | null
		error?: string | null
	}

	type CheckStatus = 'idle' | 'loading' | 'success' | 'error'

	let apiStatus: CheckStatus = $state('idle')
	let apiResult: CredentialsCheckResult | null = $state(null)

	let workerStatus: CheckStatus = $state('idle')
	let workerResult: (CredentialsCheckResult & { worker?: string }) | null = $state(null)

	async function checkApiCredentials() {
		apiStatus = 'loading'
		apiResult = null

		try {
			const response = await fetch(`/api/w/${$workspaceStore}/ai/check_bedrock_credentials`)
			if (!response.ok) {
				throw new Error(`HTTP error: ${response.status}`)
			}
			apiResult = await response.json()
			apiStatus = apiResult?.available ? 'success' : 'error'
		} catch (err) {
			apiResult = {
				available: false,
				has_session_token: false,
				error: err instanceof Error ? err.message : 'Unknown error'
			}
			apiStatus = 'error'
		}
	}

	async function checkWorkerCredentials() {
		workerStatus = 'loading'
		workerResult = null

		try {
			// Create minimal flow with AI agent dry_run step
			const flowValue = {
				modules: [
					{
						id: 'a',
						value: {
							type: 'aiagent' as const,
							input_transforms: {
								provider: {
									type: 'static' as const,
									value: {
										kind: 'aws_bedrock',
										resource: {
											api_key: 'dry_run_placeholder',
											region: 'us-east-1'
										},
										model: 'dry_run_placeholder'
									}
								},
								user_message: { type: 'static' as const, value: 'dry_run_placeholder' },
								output_type: { type: 'static' as const, value: 'text' },
								credentials_check: { type: 'static' as const, value: true }
							},
							tools: []
						}
					}
				]
			}

			const job = await JobService.runFlowPreview({
				workspace: $workspaceStore!,
				requestBody: {
					value: flowValue,
					args: {}
				}
			})

			tryEvery({
				tryCode: async () => {
					const testResult = await JobService.getCompletedJob({
						workspace: $workspaceStore!,
						id: job
					})

					if (testResult.success && testResult.result) {
						const result = testResult.result as {
							credentials_check?: boolean
							credentials?: CredentialsCheckResult
						}
						if (result?.credentials_check && result?.credentials) {
							workerResult = {
								...result.credentials,
								source: 'worker_process',
								worker: testResult.worker ?? undefined
							}
							workerStatus = workerResult.available ? 'success' : 'error'
						} else {
							workerResult = {
								available: false,
								has_session_token: false,
								error: 'Unexpected response format',
								worker: testResult.worker ?? undefined
							}
							workerStatus = 'error'
						}
					} else {
						workerResult = {
							available: false,
							has_session_token: false,
							error:
								(testResult.result as Record<string, any>)?.['error']?.['message'] ?? 'Job failed',
							worker: testResult.worker ?? undefined
						}
						workerStatus = 'error'
					}
				},
				timeoutCode: async () => {
					workerResult = {
						available: false,
						has_session_token: false,
						error: 'Timeout: job did not complete within 10s'
					}
					workerStatus = 'error'
					try {
						await JobService.cancelQueuedJob({
							workspace: $workspaceStore!,
							id: job,
							requestBody: {
								reason: 'Timeout checking Bedrock credentials'
							}
						})
					} catch (err) {
						console.error(err)
					}
				},
				interval: 500,
				timeout: 10000
			})
		} catch (err) {
			workerResult = {
				available: false,
				has_session_token: false,
				error: err instanceof Error ? err.message : 'Unknown error'
			}
			workerStatus = 'error'
		}
	}

	function checkBoth() {
		checkApiCredentials()
		checkWorkerCredentials()
	}
</script>

<div class="flex flex-col gap-3 p-3 border rounded-md bg-surface-secondary">
	<div class="flex items-center justify-between">
		<h4 class="text-sm font-semibold">AWS Environment Credentials Check</h4>
		<Button
			size="xs"
			variant="border"
			on:click={checkBoth}
			disabled={apiStatus === 'loading' || workerStatus === 'loading'}
		>
			{#if apiStatus === 'loading' || workerStatus === 'loading'}
				<LoaderCircle class="animate-spin mr-1.5 h-3.5 w-3.5" />
			{/if}
			Check Credentials
		</Button>
	</div>

	<p class="text-xs text-secondary">
		Check if AWS credentials are available from environment variables. When available, Bedrock will
		use these instead of configured credentials.
	</p>

	<div class="grid grid-cols-2 gap-3">
		<!-- API Server Check -->
		<div class="flex flex-col gap-1.5 p-2 border rounded bg-surface">
			<div class="flex items-center gap-1.5 text-xs font-medium">
				<Server class="h-3.5 w-3.5" />
				<span>API Server</span>
				{#if apiStatus === 'loading'}
					<LoaderCircle class="animate-spin h-3.5 w-3.5 ml-auto text-blue-500" />
				{:else if apiStatus === 'success'}
					<Check class="h-3.5 w-3.5 ml-auto text-green-500" />
				{:else if apiStatus === 'error'}
					<X class="h-3.5 w-3.5 ml-auto text-red-500" />
				{/if}
			</div>

			{#if apiResult}
				<div class="text-xs">
					{#if apiResult.available}
						<div class="text-green-600 dark:text-green-400">
							Available: {apiResult.access_key_id_prefix}
						</div>
						{#if apiResult.region}
							<div class="text-secondary">Region: {apiResult.region}</div>
						{/if}
						{#if apiResult.has_session_token}
							<div class="text-secondary">Session token present</div>
						{/if}
					{:else}
						<div class="text-red-600 dark:text-red-400 break-words">
							{apiResult.error ?? 'Not available'}
						</div>
					{/if}
				</div>
			{:else if apiStatus === 'idle'}
				<div class="text-xs text-tertiary">Click "Check Credentials" to test</div>
			{/if}
		</div>

		<!-- Worker Check -->
		<div class="flex flex-col gap-1.5 p-2 border rounded bg-surface">
			<div class="flex items-center gap-1.5 text-xs font-medium">
				<Cpu class="h-3.5 w-3.5" />
				<span>Worker</span>
				{#if workerStatus === 'loading'}
					<LoaderCircle class="animate-spin h-3.5 w-3.5 ml-auto text-blue-500" />
				{:else if workerStatus === 'success'}
					<Check class="h-3.5 w-3.5 ml-auto text-green-500" />
				{:else if workerStatus === 'error'}
					<X class="h-3.5 w-3.5 ml-auto text-red-500" />
				{/if}
			</div>

			{#if workerResult}
				<div class="text-xs">
					{#if workerResult.available}
						<div class="text-green-600 dark:text-green-400">
							Available: {workerResult.access_key_id_prefix}
						</div>
						{#if workerResult.region}
							<div class="text-secondary">Region: {workerResult.region}</div>
						{/if}
						{#if workerResult.has_session_token}
							<div class="text-secondary">Session token present</div>
						{/if}
						{#if workerResult.source}
							<div class="text-secondary">Source: {workerResult.source}</div>
						{/if}
						{#if workerResult.worker}
							<div class="text-secondary">Worker: {workerResult.worker}</div>
						{/if}
					{:else}
						<div class="text-red-600 dark:text-red-400 break-words">
							{workerResult.error ?? 'Not available'}
						</div>
						{#if workerResult.worker}
							<div class="text-secondary">Worker: {workerResult.worker}</div>
						{/if}
					{/if}
				</div>
			{:else if workerStatus === 'idle'}
				<div class="text-xs text-tertiary">Click "Check Credentials" to test</div>
			{/if}
		</div>
	</div>
</div>

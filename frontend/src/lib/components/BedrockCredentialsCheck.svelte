<script lang="ts">
	import { JobService, type Preview } from '$lib/gen'
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

		const pythonScript = `
import os
import boto3
from botocore.exceptions import NoCredentialsError

def main():
    # Check environment variables first
    env_access_key = os.environ.get("AWS_ACCESS_KEY_ID")
    env_region = os.environ.get("AWS_REGION") or os.environ.get("AWS_DEFAULT_REGION")
    env_session_token = os.environ.get("AWS_SESSION_TOKEN")

    # If env vars are set, return them directly
    if env_access_key:
        return {
            "available": True,
            "access_key_id_prefix": f"{env_access_key[:8]}..." if len(env_access_key) >= 8 else env_access_key,
            "region": env_region,
            "has_session_token": env_session_token is not None,
            "source": "environment",
            "error": None
        }

    # Try boto3 session as fallback
    try:
        session = boto3.Session()
        credentials = session.get_credentials()
        if credentials:
            frozen_credentials = credentials.get_frozen_credentials()
            access_key = frozen_credentials.access_key
            return {
                "available": True,
                "access_key_id_prefix": f"{access_key[:8]}..." if access_key and len(access_key) >= 8 else access_key,
                "region": session.region_name,
                "has_session_token": frozen_credentials.token is not None,
                "source": "boto3",
                "error": None
            }
        else:
            return {
                "available": False,
                "has_session_token": False,
                "error": "No credentials found in environment or boto3"
            }
    except Exception as e:
        return {
            "available": False,
            "has_session_token": False,
            "error": str(e)
        }
`

		try {
			const job = await JobService.runScriptPreview({
				workspace: $workspaceStore!,
				requestBody: {
					path: 'check_bedrock_credentials',
					language: 'python3' as Preview['language'],
					content: pythonScript,
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
						workerResult = {
							...(testResult.result as CredentialsCheckResult),
							worker: testResult.worker ?? undefined
						}
						workerStatus = workerResult.available ? 'success' : 'error'
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

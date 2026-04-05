<script lang="ts">
	import { Database, HardDrive, Loader2, Trash2 } from 'lucide-svelte'
	import { onDestroy } from 'svelte'
	import Toggle from './Toggle.svelte'
	import { Button, Tab, Tabs } from './common'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TestConnection from './TestConnection.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { displaySize } from '$lib/utils'
	import SimpleEditor from './SimpleEditor.svelte'
	import Label from './Label.svelte'
	import TextInput from './text_input/TextInput.svelte'

	type S3Config = {
		type: 'S3'
		bucket: string
		region: string
		access_key: string
		secret_key: string
		endpoint: string
		allow_http?: boolean
	}

	type AzureConfig = {
		type: 'Azure'
		accountName: string
		containerName: string
		useSSL?: boolean
		tenantId: string
		clientId: string
		accessKey: string
		endpoint?: string
	}

	type AwsOidcConfig = {
		type: 'AwsOidc'
		bucket: string
		region: string
		roleArn: string
	}

	type GcsConfig = {
		type: 'Gcs'
		bucket: string
		serviceAccountKey: Record<string, string> | undefined
	}

	interface Props {
		bucket_config?: S3Config | AzureConfig | AwsOidcConfig | GcsConfig | undefined
	}

	let {
		bucket_config = $bindable<S3Config | AzureConfig | AwsOidcConfig | GcsConfig | undefined>(
			undefined
		)
	}: Props = $props()

	let effectiveAllowHttp = $derived(
		bucket_config?.type === 'S3' ? (bucket_config.allow_http ?? true) : false
	)

	let loading = $state(false)

	async function testConnection() {
		loading = true
		try {
			if (bucket_config) {
				await SettingService.testObjectStorageConfig({
					requestBody: bucket_config
				})
				sendUserToast('Connection successful', false)
			}
		} catch (e) {
			sendUserToast(e.body, true)
		} finally {
			loading = false
		}
	}

	let usageLoading = $state(false)
	let usageError = $state<string | undefined>(undefined)
	let usageData = $state<Array<{ prefix: string; size: number }> | undefined>(undefined)

	async function loadUsage() {
		usageLoading = true
		usageError = undefined
		try {
			usageData = await SettingService.getObjectStorageUsage()
		} catch (e: any) {
			usageError = e?.body ?? e?.message ?? 'Failed to load storage usage'
			usageData = undefined
		} finally {
			usageLoading = false
		}
	}

	type CleanupStatus = {
		running: boolean
		started_at: string
		finished_at?: string | null
		total_service: number
		processed_service: number
		total_jobs: number
		processed_jobs: number
		s3_deleted: number
		errors: number
		last_error?: string | null
	}

	let cleanupStatus = $state<CleanupStatus | null | undefined>(undefined)
	let cleanupStarting = $state(false)
	let cleanupPollHandle: ReturnType<typeof setInterval> | undefined = undefined

	let cleanupProgress = $derived.by(() => {
		if (!cleanupStatus) return 0
		const total = cleanupStatus.total_service + cleanupStatus.total_jobs
		const processed = cleanupStatus.processed_service + cleanupStatus.processed_jobs
		return total > 0 ? Math.min(100, Math.round((processed / total) * 100)) : 0
	})

	async function fetchCleanupStatus() {
		try {
			cleanupStatus = (await SettingService.getLogCleanupStatus()) ?? null
		} catch (e: any) {
			// Silent — polling errors shouldn't spam toasts.
			console.warn('failed to fetch log cleanup status', e)
		}
	}

	function startPolling() {
		if (cleanupPollHandle !== undefined) return
		cleanupPollHandle = setInterval(async () => {
			await fetchCleanupStatus()
			if (cleanupStatus && !cleanupStatus.running) {
				stopPolling()
			}
		}, 1000)
	}

	function stopPolling() {
		if (cleanupPollHandle !== undefined) {
			clearInterval(cleanupPollHandle)
			cleanupPollHandle = undefined
		}
	}

	async function startCleanup() {
		cleanupStarting = true
		try {
			await SettingService.runLogCleanup()
			await fetchCleanupStatus()
			startPolling()
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed to start cleanup', true)
		} finally {
			cleanupStarting = false
		}
	}

	let hasConfig = $derived(Boolean(bucket_config))
	$effect(() => {
		if (hasConfig) {
			fetchCleanupStatus().then(() => {
				if (cleanupStatus?.running) {
					startPolling()
				}
			})
		} else {
			stopPolling()
			cleanupStatus = undefined
		}
	})

	onDestroy(stopPolling)

	let simpleEditor: SimpleEditor | undefined = $state(undefined)
	let serviceAccountKeyCode = $state(
		bucket_config?.type === 'Gcs'
			? JSON.stringify(bucket_config.serviceAccountKey, null, '\t')
			: '{}'
	)
	let lastEditorSyncedJson =
		bucket_config?.type === 'Gcs' ? JSON.stringify(bucket_config.serviceAccountKey) : '{}'

	$effect(() => {
		if (bucket_config?.type === 'Gcs') {
			const configJson = JSON.stringify(bucket_config.serviceAccountKey)
			if (configJson !== lastEditorSyncedJson) {
				lastEditorSyncedJson = configJson
				const formatted = JSON.stringify(bucket_config.serviceAccountKey, null, '\t')
				serviceAccountKeyCode = formatted
				simpleEditor?.setCode(formatted)
			}
		}
	})
</script>

<div class="my-0.5">
	<Toggle
		disabled={!$enterpriseLicense}
		options={{ right: bucket_config ? '' : 'set object store' }}
		checked={Boolean(bucket_config)}
		on:change={(e) => {
			if (e.detail) {
				bucket_config = {
					type: 'S3',
					bucket: '',
					region: '',
					access_key: '',
					secret_key: '',
					endpoint: '',
					allow_http: true
				}
			} else {
				bucket_config = undefined
			}
		}}
	/>
</div>
{#if bucket_config}
	<div class="">
		<div class="flex gap-2 py-1">
			<Button
				spacingSize="sm"
				size="xs"
				btnClasses="h-8"
				variant="default"
				on:click={testConnection}
			>
				{#if loading}
					<Loader2 class="animate-spin mr-2 !h-4 !w-4" />
				{:else}
					<Database class="mr-2 !h-4 !w-4" />
				{/if}
				Test from a server
			</Button>
			<TestConnection
				args={bucket_config}
				resourceType="s3_bucket"
				workspaceOverride="admins"
				buttonTextOverride="Test from a worker"
			/>
		</div>

		<div class="border rounded-md p-3 my-2">
			<div class="flex items-center justify-between">
				<span class="text-xs font-semibold text-emphasis">Storage usage by folder</span>
				<Button spacingSize="sm" size="xs" btnClasses="h-8" variant="border" on:click={loadUsage}>
					{#if usageLoading}
						<Loader2 class="animate-spin mr-2 !h-4 !w-4" />
					{:else}
						<HardDrive class="mr-2 !h-4 !w-4" />
					{/if}
					{usageData ? 'Refresh' : 'Show usage'}
				</Button>
			</div>

			{#if usageError}
				<div class="text-red-500 text-xs mt-2">{usageError}</div>
			{/if}

			{#if usageData}
				{#if usageData.length === 0}
					<div class="text-tertiary text-xs mt-2">No objects found in the bucket.</div>
				{:else}
					<div class="flex flex-col gap-0.5 mt-2">
						{#each usageData as item (item.prefix)}
							<div
								class="flex justify-between items-center text-xs py-1 px-2 rounded hover:bg-surface-hover"
							>
								<span class="font-mono text-secondary">{item.prefix}</span>
								<span class="text-tertiary font-semibold">{displaySize(item.size) ?? '0 B'}</span>
							</div>
						{/each}
						<div
							class="flex justify-between items-center text-xs py-1 px-2 border-t mt-1 pt-2 font-semibold"
						>
							<span>Total</span>
							<span
								>{displaySize(usageData.reduce((acc, item) => acc + item.size, 0)) ?? '0 B'}</span
							>
						</div>
					</div>
				{/if}
			{/if}
		</div>

		<div class="border rounded-md p-3 my-2">
			<div class="flex items-center justify-between gap-2">
				<div class="flex flex-col">
					<span class="text-xs font-semibold text-emphasis">Clean up expired logs</span>
					<span class="text-tertiary text-2xs">
						Delete expired service &amp; job logs from object storage and disk now. Uses batched
						deletes (up to 1000 objects per request).
					</span>
				</div>
				<Button
					spacingSize="sm"
					size="xs"
					btnClasses="h-8"
					variant="border"
					disabled={cleanupStarting || cleanupStatus?.running}
					on:click={startCleanup}
				>
					{#if cleanupStarting || cleanupStatus?.running}
						<Loader2 class="animate-spin mr-2 !h-4 !w-4" />
					{:else}
						<Trash2 class="mr-2 !h-4 !w-4" />
					{/if}
					{cleanupStatus?.running ? 'Running…' : 'Run cleanup'}
				</Button>
			</div>

			{#if cleanupStatus}
				{@const total = cleanupStatus.total_service + cleanupStatus.total_jobs}
				{@const processed = cleanupStatus.processed_service + cleanupStatus.processed_jobs}
				<div class="mt-3 flex flex-col gap-1">
					<div class="w-full h-2 bg-surface-secondary rounded overflow-hidden">
						<div class="h-full bg-blue-500 transition-all" style:width="{cleanupProgress}%"></div>
					</div>
					<div class="flex justify-between text-2xs text-tertiary">
						<span>
							{processed.toLocaleString()} / {total.toLocaleString()} files ({cleanupProgress}%)
						</span>
						<span>
							S3 deleted: {cleanupStatus.s3_deleted.toLocaleString()}
							{#if cleanupStatus.errors > 0}
								&middot; errors: {cleanupStatus.errors.toLocaleString()}
							{/if}
						</span>
					</div>
					<div class="text-2xs text-tertiary">
						Service logs: {cleanupStatus.processed_service.toLocaleString()} / {cleanupStatus.total_service.toLocaleString()}
						&middot; Job logs: {cleanupStatus.processed_jobs.toLocaleString()} / {cleanupStatus.total_jobs.toLocaleString()}
					</div>
					{#if !cleanupStatus.running && cleanupStatus.finished_at}
						<div class="text-2xs text-tertiary">
							Finished at {new Date(cleanupStatus.finished_at).toLocaleString()}
						</div>
					{/if}
					{#if cleanupStatus.last_error}
						<div class="text-red-500 text-2xs mt-1">
							Last error: {cleanupStatus.last_error}
						</div>
					{/if}
				</div>
			{/if}
		</div>

		<Tabs
			selected={bucket_config?.type ?? 'S3'}
			on:selected={(e) => {
				if (e.detail === 'S3' && bucket_config?.type !== 'S3') {
					bucket_config = {
						type: 'S3',
						bucket: '',
						region: '',
						access_key: '',
						secret_key: '',
						endpoint: '',
						allow_http: true
					}
				} else if (e.detail === 'Azure' && bucket_config?.type !== 'Azure') {
					bucket_config = {
						type: 'Azure',
						accountName: '',
						containerName: '',
						useSSL: false,
						tenantId: '',
						clientId: '',
						accessKey: ''
					}
				} else if (e.detail === 'Gcs' && bucket_config?.type !== 'Gcs') {
					bucket_config = {
						type: 'Gcs',
						bucket: '',
						serviceAccountKey: {}
					}
				} else if (e.detail === 'AwsOidc' && bucket_config?.type !== 'AwsOidc') {
					bucket_config = {
						type: 'AwsOidc',
						bucket: '',
						region: '',
						roleArn: ''
					}
				}
			}}
		>
			<Tab value="S3" label="S3" />
			<Tab value="Azure" label="Azure Blob" />
			<Tab value="AwsOidc" label="AWS OIDC" />
			<Tab value="Gcs" label="Google Cloud Storage" />
		</Tabs>
		<div class="flex flex-col gap-6 mt-2 p-4 border rounded-md">
			{#if bucket_config.type === 'S3'}
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Bucket</span>
					<TextInput
						inputProps={{ placeholder: 'bucket-name' }}
						bind:value={
							() => (bucket_config as S3Config).bucket,
							(v) => (bucket_config = { ...(bucket_config as S3Config), bucket: v })
						}
					/>
				</label>

				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Region</span>
					<span class="text-primary text-2xs"
						>If left empty, will be derived automatically from $AWS_REGION</span
					>
					<TextInput
						bind:value={
							() => (bucket_config as S3Config).region,
							(v) => (bucket_config = { ...(bucket_config as S3Config), region: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Access key ID</span>
					<span class="text-primary text-2xs"
						>If left empty, will be derived automatically from $AWS_ACCESS_KEY_ID, pod or ec2
						profile</span
					>
					<TextInput
						bind:value={
							() => (bucket_config as S3Config).access_key,
							(v) => (bucket_config = { ...(bucket_config as S3Config), access_key: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Secret key</span>
					<span class="text-primary text-2xs"
						>If left empty, will be derived automatically from $AWS_SECRET_KEY, pod or ec2 profile</span
					>
					<TextInput
						inputProps={{ type: 'password', autocomplete: 'new-password' }}
						bind:value={
							() => (bucket_config as S3Config).secret_key,
							(v) => (bucket_config = { ...(bucket_config as S3Config), secret_key: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Endpoint</span>
					<span class="text-primary text-2xs"
						>Only needed for non AWS S3 providers like R2 or MinIo</span
					>
					<TextInput
						bind:value={
							() => (bucket_config as S3Config).endpoint,
							(v) => (bucket_config = { ...(bucket_config as S3Config), endpoint: v })
						}
					/>
				</label>
				<div class="block pb-2">
					<span class="text-primary text-2xs">Disable if using https only policy</span>
					<div>
						<Toggle
							checked={effectiveAllowHttp}
							on:change={(e) => {
								if (bucket_config?.type === 'S3') {
									bucket_config = { ...bucket_config, allow_http: e.detail }
								}
							}}
							options={{ right: 'Allow http' }}
						/>
					</div>
				</div>
			{:else if bucket_config.type === 'Azure'}
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Account name</span>
					<TextInput
						inputProps={{ placeholder: 'account-name' }}
						bind:value={
							() => (bucket_config as AzureConfig).accountName,
							(v) => (bucket_config = { ...(bucket_config as AzureConfig), accountName: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Container name</span>
					<TextInput
						inputProps={{ placeholder: 'container-name' }}
						bind:value={
							() => (bucket_config as AzureConfig).containerName,
							(v) => (bucket_config = { ...(bucket_config as AzureConfig), containerName: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Access key</span>
					<TextInput
						inputProps={{ type: 'password', autocomplete: 'new-password' }}
						bind:value={
							() => (bucket_config as AzureConfig).accessKey,
							(v) => (bucket_config = { ...(bucket_config as AzureConfig), accessKey: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis"
						>Tenant ID <span class="text-2xs text-primary">(optional)</span></span
					>
					<TextInput
						bind:value={
							() => (bucket_config as AzureConfig).tenantId,
							(v) => (bucket_config = { ...(bucket_config as AzureConfig), tenantId: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis"
						>Client ID <span class="text-2xs text-primary">(optional)</span></span
					>
					<TextInput
						bind:value={
							() => (bucket_config as AzureConfig).clientId,
							(v) => (bucket_config = { ...(bucket_config as AzureConfig), clientId: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis"
						>Endpoint <span class="text-2xs text-primary">(optional)</span></span
					>
					<span class="text-primary text-2xs"
						>Only needed for non Azure Blob providers like Azurite</span
					>
					<TextInput
						bind:value={
							() => (bucket_config as AzureConfig).endpoint,
							(v) => (bucket_config = { ...(bucket_config as AzureConfig), endpoint: v })
						}
					/>
				</label>
			{:else if bucket_config.type === 'AwsOidc'}
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Bucket</span>
					<TextInput
						inputProps={{ placeholder: 'bucket-name' }}
						bind:value={
							() => (bucket_config as AwsOidcConfig).bucket,
							(v) => (bucket_config = { ...(bucket_config as AwsOidcConfig), bucket: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Region</span>
					<TextInput
						inputProps={{ placeholder: 'region' }}
						bind:value={
							() => (bucket_config as AwsOidcConfig).region,
							(v) => (bucket_config = { ...(bucket_config as AwsOidcConfig), region: v })
						}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-xs font-semibold text-emphasis">Role ARN</span>
					<TextInput
						inputProps={{ placeholder: 'arn:aws:iam::123456789012:role/test' }}
						bind:value={
							() => (bucket_config as AwsOidcConfig).roleArn,
							(v) => (bucket_config = { ...(bucket_config as AwsOidcConfig), roleArn: v })
						}
					/>
				</label>
			{:else if bucket_config.type === 'Gcs'}
				<Label label="Bucket">
					<TextInput
						inputProps={{ placeholder: 'bucket-name' }}
						bind:value={
							() => (bucket_config as GcsConfig).bucket,
							(v) => (bucket_config = { ...(bucket_config as GcsConfig), bucket: v })
						}
					/>
				</Label>
				<Label label="Service Account Key">
					<span class="text-primary text-2xs">JSON content of the service account key file</span>
					<SimpleEditor
						bind:this={simpleEditor}
						lang="json"
						bind:code={serviceAccountKeyCode}
						on:change={(e) => {
							if (bucket_config?.type === 'Gcs') {
								if (e.detail.code === undefined || e.detail.code === '') {
									bucket_config = { ...bucket_config, serviceAccountKey: undefined }
									return
								}
								try {
									const parsed = JSON.parse(e.detail.code ?? '{}')
									lastEditorSyncedJson = JSON.stringify(parsed)
									bucket_config = { ...bucket_config, serviceAccountKey: parsed }
								} catch (_) {
									bucket_config = { ...bucket_config, serviceAccountKey: undefined }
								}
							}
						}}
						class="h-80"
					/>
				</Label>
			{:else}
				<div>Unknown bucket type {bucket_config['type']}</div>
			{/if}
		</div>
	</div>
{/if}

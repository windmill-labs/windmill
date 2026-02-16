<script lang="ts">
	import { Database, Loader2 } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import { Button, Tab, Tabs } from './common'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TestConnection from './TestConnection.svelte'
	import { enterpriseLicense } from '$lib/stores'
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

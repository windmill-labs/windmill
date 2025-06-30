<script lang="ts">
	import { Database, Loader2 } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import { Button, Tab, Tabs } from './common'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TestConnection from './TestConnection.svelte'
	import { enterpriseLicense } from '$lib/stores'

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
		region?: string
		serviceAccountKey?: string
		serviceAccountKeyPath?: string
		endpoint?: string
		useSSL?: boolean
	}

	export let bucket_config: S3Config | AzureConfig | AwsOidcConfig | GcsConfig | undefined = undefined

	$: bucket_config?.type == 'S3' &&
		bucket_config.allow_http == undefined &&
		(bucket_config.allow_http = true)
	let loading = false

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
					endpoint: ''
				}
			} else {
				bucket_config = undefined
			}
		}}
	/>
</div>
{#if bucket_config}
	<div class="p-2">
		<div class="flex gap-2 py-1">
			<Button
				spacingSize="sm"
				size="xs"
				btnClasses="h-8"
				color="light"
				variant="border"
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
			bind:selected={bucket_config.type}
			on:selected={(e) => {
				if (e.detail === 'S3') {
					bucket_config = {
						type: 'S3',
						bucket: '',
						region: '',
						access_key: '',
						secret_key: '',
						endpoint: ''
					}
				} else if (e.detail === 'Azure') {
					bucket_config = {
						type: 'Azure',
						accountName: '',
						containerName: '',
						useSSL: false,
						tenantId: '',
						clientId: '',
						accessKey: ''
					}
				} else if (e.detail === 'Gcs') {
					bucket_config = {
						type: 'Gcs',
						bucket: '',
						region: '',
						serviceAccountKey: '',
						serviceAccountKeyPath: '',
						endpoint: '',
						useSSL: true
					}
				}
			}}
		>
			<Tab size="sm" value="S3">S3</Tab>
			<Tab size="sm" value="Azure">Azure Blob</Tab>
			<Tab size="sm" value="AwsOidc">AWS OIDC</Tab>
			<Tab size="sm" value="Gcs">GCS</Tab>
		</Tabs>
		<div class="flex flex-col gap-2 mt-2 p-2 border rounded-md">
			{#if bucket_config.type === 'S3'}
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Bucket</span>
					<input type="text" placeholder="bucket-name" bind:value={bucket_config.bucket} />
				</label>

				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Region</span>
					<span class="text-tertiary text-2xs"
						>If left empty, will be derived automatically from $AWS_REGION</span
					>
					<input type="text" bind:value={bucket_config.region} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Access key ID</span>
					<span class="text-tertiary text-2xs"
						>If left empty, will be derived automatically from $AWS_ACCESS_KEY_ID, pod or ec2
						profile</span
					>
					<input type="text" bind:value={bucket_config.access_key} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Secret key</span>
					<span class="text-tertiary text-2xs"
						>If left empty, will be derived automatically from $AWS_SECRET_KEY, pod or ec2 profile</span
					>
					<input
						type="password"
						autocomplete="new-password"
						bind:value={bucket_config.secret_key}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Endpoint</span>
					<span class="text-tertiary text-2xs"
						>Only needed for non AWS S3 providers like R2 or MinIo</span
					>
					<input type="text" bind:value={bucket_config.endpoint} />
				</label>
				<div class="block pb-2">
					<span class="text-tertiary text-2xs">Disable if using https only policy</span>
					<div>
						<Toggle bind:checked={bucket_config.allow_http} options={{ right: 'Allow http' }} />
					</div>
				</div>
			{:else if bucket_config.type === 'Azure'}
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Account name</span>
					<input type="text" placeholder="account-name" bind:value={bucket_config.accountName} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Container name</span>
					<input
						type="text"
						placeholder="container-name"
						bind:value={bucket_config.containerName}
					/>
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Access key</span>
					<input type="password" autocomplete="new-password" bind:value={bucket_config.accessKey} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm"
						>Tenant ID <span class="text-2xs text-tertiary">(optional)</span></span
					>
					<input type="text" bind:value={bucket_config.tenantId} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm"
						>Client ID <span class="text-2xs text-tertiary">(optional)</span></span
					>
					<input type="text" bind:value={bucket_config.clientId} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm"
						>Endpoint <span class="text-2xs text-tertiary">(optional)</span></span
					>
					<span class="text-tertiary text-2xs"
						>Only needed for non Azure Blob providers like Azurite</span
					>
					<input type="text" bind:value={bucket_config.endpoint} />
				</label>
			{:else if bucket_config.type === 'AwsOidc'}
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Bucket</span>
					<input type="text" placeholder="bucket-name" bind:value={bucket_config.bucket} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Region</span>
					<input type="text" placeholder="region" bind:value={bucket_config.region} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Role ARN</span>
					<input
						type="text"
						placeholder="arn:aws:iam::123456789012:role/test"
						bind:value={bucket_config.roleArn}
					/>
				</label>
			{:else if bucket_config.type === 'Gcs'}
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Bucket</span>
					<input type="text" placeholder="bucket-name" bind:value={bucket_config.bucket} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Region</span>
					<span class="text-tertiary text-2xs"
						>If left empty, will use default region</span
					>
					<input type="text" placeholder="us-central1" bind:value={bucket_config.region} />
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Service Account Key</span>
					<span class="text-tertiary text-2xs"
						>JSON content of the service account key file (optional)</span
					>
					<textarea 
						rows="3" 
						placeholder="Paste service account JSON key here..."
						bind:value={bucket_config.serviceAccountKey} 
					></textarea>
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Service Account Key Path</span>
					<span class="text-tertiary text-2xs"
						>Path to service account key file on the server (optional)</span
					>
					<input 
						type="text" 
						placeholder="/path/to/service-account-key.json"
						bind:value={bucket_config.serviceAccountKeyPath} 
					/>
				</label>
				<label class="block pb-2">
					<span class="text-primary font-semibold text-sm">Endpoint</span>
					<span class="text-tertiary text-2xs"
						>Custom endpoint for GCS-compatible storage (optional)</span
					>
					<input type="text" placeholder="https://storage.googleapis.com" bind:value={bucket_config.endpoint} />
				</label>
				<div class="block pb-2">
					<span class="text-tertiary text-2xs">Enable HTTPS for secure connections</span>
					<div>
						<Toggle bind:checked={bucket_config.useSSL} options={{ right: 'Use SSL' }} />
					</div>
				</div>
			{:else}
				<div>Unknown bucket type {bucket_config['type']}</div>
			{/if}
		</div>
	</div>
{/if}

<script lang="ts">
	import { Database, Loader2 } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import { Button, Tab, Tabs } from './common'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TestConnection from './TestConnection.svelte'

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

	export let bucket_config: S3Config | AzureConfig | undefined = undefined

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

<div>
	<Toggle
		options={{ right: 'Enable' }}
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
	<div class="flex gap-2">
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
			}
		}}
	>
		<Tab size="sm" value="S3">S3</Tab>
		<Tab size="sm" value="Azure">Azure Blob</Tab>
	</Tabs>
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
			<span class="text-primary font-semibold text-sm">Access Key ID</span>
			<span class="text-tertiary text-2xs"
				>If left empty, will be derived automatically from $AWS_ACCESS_KEY_ID, pod or ec2 profile</span
			>
			<input type="text" bind:value={bucket_config.access_key} />
		</label>
		<label class="block pb-2">
			<span class="text-primary font-semibold text-sm">Secret Key</span>
			<span class="text-tertiary text-2xs"
				>If left empty, will be derived automatically from $AWS_SECRET_KEY, pod or ec2 profile</span
			>
			<input type="password" bind:value={bucket_config.secret_key} />
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
				<Toggle bind:checked={bucket_config.allow_http} options={{ right: 'allow http' }} />
			</div>
		</div>
	{:else if bucket_config.type === 'Azure'}
		<label class="block pb-2">
			<span class="text-primary font-semibold text-sm">Account Name</span>
			<input type="text" placeholder="account-name" bind:value={bucket_config.accountName} />
		</label>
		<label class="block pb-2">
			<span class="text-primary font-semibold text-sm">Container Name</span>
			<input type="text" placeholder="container-name" bind:value={bucket_config.containerName} />
		</label>
		<label class="block pb-2">
			<span class="text-primary font-semibold text-sm">Access Key</span>
			<input type="password" bind:value={bucket_config.accessKey} />
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
	{:else}
		<div>Unknown bucket type {bucket_config['type']}</div>
	{/if}
{/if}

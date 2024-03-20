<script lang="ts">
	import { Database, Loader2 } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import { Button } from './common'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TestConnection from './TestConnection.svelte'

	type BucketConfig = {
		bucket: string
		region: string
		access_key: string
		secret_key: string
		endpoint: string
	}
	export let bucket_config: BucketConfig | undefined = undefined

	let loading = false

	async function testConnection() {
		loading = true
		try {
			if (bucket_config) {
				await SettingService.testS3Config({ requestBody: bucket_config })
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
		<input type="text" bind:value={bucket_config.secret_key} />
	</label>
	<label class="block pb-2">
		<span class="text-primary font-semibold text-sm">Endpoint</span>
		<span class="text-tertiary text-2xs">Only needed for non AWS S3 providers like R2 or MinIo</span
		>
		<input type="text" bind:value={bucket_config.endpoint} />
	</label>
{/if}

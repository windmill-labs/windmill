<script lang="ts">
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { emptyString, sendUserToast } from '$lib/utils'
	import { Plus, X } from 'lucide-svelte'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import Tabs from '../common/tabs/Tabs.svelte'
	import Description from '../Description.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { convertFrontendToBackendSetting, type S3ResourceSettings } from '$lib/workspace_settings'
	import { WorkspaceService } from '$lib/gen'
	import S3FilePicker from '../S3FilePicker.svelte'
	import Portal from '../Portal.svelte'
	import { fade } from 'svelte/transition'

	let { s3ResourceSettings = $bindable() }: { s3ResourceSettings: S3ResourceSettings } = $props()

	let s3FileViewer: S3FilePicker | undefined = $state()

	async function editWindmillLFSSettings(): Promise<void> {
		const large_file_storage = convertFrontendToBackendSetting(s3ResourceSettings)
		await WorkspaceService.editLargeFileStorageConfig({
			workspace: $workspaceStore!,
			requestBody: {
				large_file_storage: large_file_storage
			}
		})
		console.log('Large file storage settings changed', large_file_storage)
		sendUserToast(`Large file storage settings changed`)
	}
</script>

<Portal name="workspace-settings">
	<S3FilePicker bind:this={s3FileViewer} readOnlyMode={false} fromWorkspaceSettings={true} />
</Portal>

<div class="flex flex-col gap-4 my-8">
	<div class="flex flex-col gap-1">
		<div class="text-primary text-lg font-semibold">Workspace Object Storage (S3/Azure Blob/GCS)</div>
		<Description
			link="https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill#workspace-object-storage"
		>
			Connect your Windmill workspace to your S3 bucket, Azure Blob storage, or Google Cloud Storage to enable users
			to read and write from object storage without having to have access to the credentials.
		</Description>
	</div>
</div>
{#if !$enterpriseLicense}
	<Alert type="info" title="S3 storage is limited to 20 files in Windmill CE">
		Windmill S3 bucket browser will not work for buckets containing more than 20 files and uploads
		are limited to files {'<'} 50MB. Consider upgrading to Windmill EE to use this feature with large
		buckets.
	</Alert>
{:else}
	<Alert type="info" title="Logs storage is set at the instance level">
		This setting is only for storage of large files allowing to upload files directly to object
		storage using S3Object and use the wmill sdk to read and write large files backed by an object
		storage. Large-scale log management and distributed dependency caching is under <a
			href="https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill#instance-object-storage"
			class="text-blue-500">Instance object storage</a
		>, set by the superadmins in the instance settings UI.
	</Alert>
{/if}
{#if s3ResourceSettings}
	<div class="mt-5">
		<div class="w-full">
			<!-- this can be removed once parent moves to runes -->
			<!-- svelte-ignore binding_property_non_reactive -->
			<Tabs bind:selected={s3ResourceSettings.resourceType}>
				<Tab exact size="xs" value="s3">S3</Tab>
				<Tab size="xs" value="azure_blob">Azure Blob</Tab>
				<Tab exact size="xs" value="s3_aws_oidc">AWS OIDC</Tab>
				<Tab size="xs" value="azure_workload_identity">Azure Workload Identity</Tab>
				<Tab exact size="xs" value="gcs">GCS</Tab>
			</Tabs>
		</div>
		<div class="w-full flex gap-1 mt-4">
			<!-- this can be removed once parent moves to runes -->
			<!-- svelte-ignore binding_property_non_reactive -->
			<ResourcePicker
				resourceType={s3ResourceSettings.resourceType}
				bind:value={s3ResourceSettings.resourcePath}
			/>
			<Button
				size="sm"
				variant="contained"
				color="dark"
				disabled={emptyString(s3ResourceSettings.resourcePath)}
				on:click={async () => {
					if ($workspaceStore) {
						s3FileViewer?.open?.(undefined)
					}
				}}>Browse content (save first)</Button
			>
		</div>
	</div>
	{#if s3ResourceSettings.resourceType == 's3'}
		<div class="flex flex-col mt-5 mb-1 gap-1">
			<!-- this can be removed once parent moves to runes -->
			<!-- svelte-ignore binding_property_non_reactive -->
			<Toggle
				disabled={emptyString(s3ResourceSettings.resourcePath)}
				bind:checked={s3ResourceSettings.publicResource}
				options={{
					right: 'S3 resource details and content can be accessed by all users of this workspace',
					rightTooltip:
						'If set, all users of this workspace will have access the to entire content of the S3 bucket, as well as the resource details and the "open preview" button. This effectively by-pass the permissions set on the resource and makes it public to everyone.'
				}}
			/>
			{#if s3ResourceSettings.publicResource === true}
				<div class="pt-2"></div>

				<Alert type="warning" title="S3 bucket content and resource details are shared">
					S3 resource public access is ON, which means that the entire content of the S3 bucket will
					be accessible to all the users of this workspace regardless of whether they have access
					the resource or not. Similarly, certain Windmill SDK endpoints can be used in scripts to
					access the resource details, including public and private keys.
				</Alert>
			{/if}
		</div>
	{:else}
		<div class="flex flex-col mt-5 mb-1 gap-1">
			<!-- this can be removed once parent moves to runes -->
			<!-- svelte-ignore binding_property_non_reactive -->
			<Toggle
				disabled={emptyString(s3ResourceSettings.resourcePath)}
				bind:checked={s3ResourceSettings.publicResource}
				options={{
					right: 'object storage content can be accessed by all users of this workspace',
					rightTooltip:
						'If set, all users of this workspace will have access the to entire content of the object storage.'
				}}
			/>
			{#if s3ResourceSettings.publicResource === true}
				<div class="pt-2"></div>
				<Alert type="warning" title="object content">
					object public access is ON, which means that the entire content of the object store will
					be accessible to all the users of this workspace regardless of whether they have access
					the resource or not.
				</Alert>
			{/if}
		</div>
	{/if}
	<div class="mt-6">
		<div class="flex mt-2 flex-col gap-y-4 max-w-3xl">
			{#each s3ResourceSettings.secondaryStorage ?? [] as _, idx}
				<div class="flex gap-1 items-center">
					<input
						class="max-w-[200px]"
						type="text"
						bind:value={
							() => s3ResourceSettings.secondaryStorage?.[idx]?.[0] || '',
							(v) => {
								if (s3ResourceSettings.secondaryStorage?.[idx]) {
									s3ResourceSettings.secondaryStorage[idx][0] = v
								}
							}
						}
						placeholder="Storage name"
					/>
					<select
						class="max-w-[125px]"
						bind:value={
							() => s3ResourceSettings.secondaryStorage?.[idx]?.[1].resourceType || 's3',
							(v) => {
								if (s3ResourceSettings.secondaryStorage?.[idx]) {
									s3ResourceSettings.secondaryStorage[idx][1].resourceType = v
								}
							}
						}
					>
						<option value="s3">S3</option>
						<option value="azure_blob">Azure Blob</option>
						<option value="s3_aws_oidc">AWS OIDC</option>
						<option value="azure_workload_identity">Azure Workload Identity</option>
					</select>
					<!-- this can be removed once parent moves to runes -->
					<!-- svelte-ignore binding_property_non_reactive -->
					<ResourcePicker
						resourceType={s3ResourceSettings.secondaryStorage?.[idx]?.[1].resourceType || 's3'}
						bind:value={
							() => s3ResourceSettings.secondaryStorage?.[idx]?.[1].resourcePath || '',
							(v) => {
								if (s3ResourceSettings.secondaryStorage?.[idx]) {
									s3ResourceSettings.secondaryStorage[idx][1].resourcePath = v
								}
							}
						}
					/>
					<Button
						size="sm"
						variant="contained"
						color="dark"
						disabled={emptyString(s3ResourceSettings.secondaryStorage?.[idx]?.[1].resourcePath)}
						on:click={async () => {
							if ($workspaceStore) {
								s3FileViewer?.open?.({
									s3: '',
									storage: s3ResourceSettings.secondaryStorage?.[idx]?.[0] || ''
								})
							}
						}}>Browse content (save first)</Button
					>
					<button
						transition:fade|local={{ duration: 100 }}
						class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover ml-2"
						aria-label="Clear"
						onclick={() => {
							if (s3ResourceSettings.secondaryStorage) {
								s3ResourceSettings.secondaryStorage.splice(idx, 1)
								s3ResourceSettings.secondaryStorage = [...s3ResourceSettings.secondaryStorage]
							}
						}}
					>
						<X size={14} />
					</button>
				</div>
			{/each}
			<div class="flex gap-1">
				<Button
					size="xs"
					variant="border"
					on:click={() => {
						if (s3ResourceSettings.secondaryStorage === undefined) {
							s3ResourceSettings.secondaryStorage = []
						}
						s3ResourceSettings.secondaryStorage.push([
							`storage_${s3ResourceSettings.secondaryStorage.length + 1}`,
							{ resourcePath: '', resourceType: 's3', publicResource: false }
						])
						s3ResourceSettings.secondaryStorage = s3ResourceSettings.secondaryStorage
					}}><Plus size={14} />Add secondary storage</Button
				>
				<Tooltip>
					Secondary storage is a feature that allows you to read and write from storage that isn't
					your main storage by specifying it in the s3 object as "secondary_storage" with the name
					of it
				</Tooltip>
			</div>
		</div>
	</div>
	<div class="flex mt-5 mb-5 gap-1">
		<Button
			on:click={() => {
				editWindmillLFSSettings()
				console.log('Saving S3 settings', s3ResourceSettings)
			}}>Save storage settings</Button
		>
	</div>
{/if}

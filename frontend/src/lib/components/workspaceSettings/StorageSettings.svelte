<script lang="ts">
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { emptyString, sendUserToast } from '$lib/utils'
	import { ChevronDown, Plus, Shield } from 'lucide-svelte'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import Tabs from '../common/tabs/Tabs.svelte'
	import Description from '../Description.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'
	import {
		convertFrontendToBackendSetting,
		defaultS3AdvancedPermissions,
		type S3ResourceSettings,
		type S3ResourceSettingsItem
	} from '$lib/workspace_settings'
	import { WorkspaceService } from '$lib/gen'
	import S3FilePicker from '../S3FilePicker.svelte'
	import Portal from '../Portal.svelte'
	import Popover from '../meltComponents/Popover.svelte'
	import ClearableInput from '../common/clearableInput/ClearableInput.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'

	let {
		s3ResourceSettings = $bindable(),
		onSave = undefined
	}: { s3ResourceSettings: S3ResourceSettings; onSave?: () => void } = $props()

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
		onSave?.()
	}
</script>

<Portal name="workspace-settings">
	<S3FilePicker bind:this={s3FileViewer} readOnlyMode={false} fromWorkspaceSettings={true} />
</Portal>

<div class="flex flex-col gap-4 my-8">
	<div class="flex flex-col gap-1">
		<div class="text-sm font-semibold text-emphasis"
			>Workspace Object Storage (S3/Azure Blob/GCS)</div
		>
		<Description
			link="https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill#workspace-object-storage"
		>
			Connect your Windmill workspace to your S3 bucket, Azure Blob storage, or Google Cloud Storage
			to enable users to read and write from object storage without having to have access to the
			credentials.
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
			>Instance object storage</a
		>, set by the superadmins in the instance settings UI.
	</Alert>
{/if}
{#if s3ResourceSettings}
	<div class="mt-5">
		<div class="w-full">
			<!-- this can be removed once parent moves to runes -->
			<!-- svelte-ignore binding_property_non_reactive -->
			<Tabs bind:selected={s3ResourceSettings.resourceType}>
				<Tab exact label="S3" value="s3" />
				<Tab value="azure_blob" label="Azure Blob" />
				<Tab exact value="s3_aws_oidc" label="AWS OIDC" />
				<Tab value="azure_workload_identity" label="Azure Workload Identity" />
				<Tab exact value="gcloud_storage" label="Google Cloud Storage" />
			</Tabs>
		</div>
		<div class="w-full flex gap-1 mt-4 whitespace-nowrap">
			<!-- this can be removed once parent moves to runes -->
			<!-- svelte-ignore binding_property_non_reactive -->
			<ResourcePicker
				resourceType={s3ResourceSettings.resourceType}
				bind:value={s3ResourceSettings.resourcePath}
			/>
			{@render permissionBtn(s3ResourceSettings)}
			<Button
				size="sm"
				variant="accent"
				disabled={emptyString(s3ResourceSettings.resourcePath)}
				on:click={async () => {
					if ($workspaceStore) {
						s3FileViewer?.open?.(undefined)
					}
				}}>Browse content (save first)</Button
			>
		</div>
	</div>

	<div class="mt-6">
		<div class="flex mt-2 flex-col gap-y-4 max-w-5xl">
			{#each s3ResourceSettings.secondaryStorage ?? [] as _, idx}
				<div class="flex gap-1 relative whitespace-nowrap">
					<TextInput
						class="max-w-[200px]"
						inputProps={{ type: 'text', placeholder: 'Storage name' }}
						bind:value={
							() => s3ResourceSettings.secondaryStorage?.[idx]?.[0] || '',
							(v) => {
								if (s3ResourceSettings.secondaryStorage?.[idx]) {
									s3ResourceSettings.secondaryStorage[idx][0] = v
								}
							}
						}
					/>
					<Select
						class="max-w-[125px]"
						inputClass="h-full"
						bind:value={
							() => s3ResourceSettings.secondaryStorage?.[idx]?.[1].resourceType || 's3',
							(v) => {
								if (s3ResourceSettings.secondaryStorage?.[idx]) {
									s3ResourceSettings.secondaryStorage[idx][1].resourceType = v
								}
							}
						}
						items={[
							{ value: 's3', label: 'S3' },
							{ value: 'azure_blob', label: 'Azure Blob' },
							{ value: 's3_aws_oidc', label: 'AWS OIDC' },
							{ value: 'azure_workload_identity', label: 'Azure Workload Identity' },
							{ value: 'gcloud_storage', label: 'Google Cloud Storage' }
						]}
					/>

					<ResourcePicker
						resourceType={s3ResourceSettings.secondaryStorage?.[idx]?.[1].resourceType || 's3'}
						bind:value={
							() => s3ResourceSettings.secondaryStorage?.[idx]?.[1].resourcePath || undefined,
							(v) => {
								if (s3ResourceSettings.secondaryStorage?.[idx]) {
									s3ResourceSettings.secondaryStorage[idx][1].resourcePath = v
								}
							}
						}
					/>
					{@render permissionBtn(s3ResourceSettings.secondaryStorage![idx][1])}
					<Button
						size="sm"
						variant="accent"
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
					<CloseButton
						class="my-auto"
						small
						on:close={() => {
							if (s3ResourceSettings.secondaryStorage) {
								s3ResourceSettings.secondaryStorage.splice(idx, 1)
								s3ResourceSettings.secondaryStorage = [...s3ResourceSettings.secondaryStorage]
							}
						}}
					/>
				</div>
			{/each}
			<div class="flex gap-1">
				<Button
					size="xs"
					variant="default"
					on:click={() => {
						if (s3ResourceSettings.secondaryStorage === undefined) {
							s3ResourceSettings.secondaryStorage = []
						}
						s3ResourceSettings.secondaryStorage.push([
							`storage_${s3ResourceSettings.secondaryStorage.length + 1}`,
							{
								resourcePath: '',
								resourceType: 's3',
								publicResource: false,
								advancedPermissions: defaultS3AdvancedPermissions(!!$enterpriseLicense)
							}
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
			variant="accent"
			size="xl"
			on:click={() => {
				editWindmillLFSSettings()
				console.log('Saving S3 settings', s3ResourceSettings)
			}}>Save storage settings</Button
		>
	</div>
{/if}

{#snippet permissionBtn(storage: NonNullable<S3ResourceSettings['secondaryStorage']>[number][1])}
	<Popover closeOnOtherPopoverOpen placement="left">
		<svelte:fragment slot="trigger">
			<Button variant="default" wrapperClasses="h-full" btnClasses="px-2.5" size="sm">
				<Shield size={16} /> Permissions <ChevronDown size={14} />
			</Button>
		</svelte:fragment>
		<svelte:fragment slot="content">
			<div class="flex flex-col gap-3 mx-4 pb-4 pt-5 w-[48rem]">
				{#if !$enterpriseLicense}
					<Alert
						type={storage.advancedPermissions ? 'error' : 'info'}
						title="Advanced permission rules are an Enterprise feature"
					>
						Consider upgrading to Windmill EE to use advanced permission rules to control access to
						your object storage at a more granular level.</Alert
					>
				{/if}
				<Toggle
					bind:checked={
						() => !!storage.advancedPermissions,
						(v) => {
							storage.advancedPermissions = v
								? defaultS3AdvancedPermissions(!!$enterpriseLicense)
								: undefined
							if (v) storage.publicResource = false
						}
					}
					options={{
						right: 'Enable advanced permission rules',
						rightTooltip: 'Control precisely which paths are allowed to your users.'
					}}
					disabled={!storage.advancedPermissions && !$enterpriseLicense}
				/>
				{#if storage.advancedPermissions}
					{@render advancedPermissionsEditor(storage.advancedPermissions)}
				{/if}
				{#if !storage.advancedPermissions}
					{#if storage.resourceType == 's3'}
						<div class="flex flex-col mt-2 mb-1 gap-1">
							<Toggle
								disabled={emptyString(storage.resourcePath)}
								bind:checked={storage.publicResource}
								options={{
									right:
										'S3 resource details and content can be accessed by all users of this workspace',
									rightTooltip:
										'If set, all users of this workspace will have access the to entire content of the S3 bucket, as well as the resource details and the "open preview" button. This effectively by-pass the permissions set on the resource and makes it public to everyone.'
								}}
							/>
							{#if storage.publicResource === true}
								<div class="pt-2"></div>

								<Alert
									type="warning"
									title="(Legacy) S3 bucket content and resource details are shared"
								>
									S3 resource public access is ON, which means that the entire content of the S3
									bucket will be accessible to all the users of this workspace regardless of whether
									they have access the resource or not. Similarly, certain Windmill SDK endpoints
									can be used in scripts to access the resource details, including public and
									private keys.
								</Alert>
							{/if}
						</div>
					{:else}
						<div class="flex flex-col mt-5 mb-1 gap-1 max-w-[40rem]">
							<Toggle
								disabled={emptyString(storage.resourcePath)}
								bind:checked={storage.publicResource}
								options={{
									right: 'object storage content can be accessed by all users of this workspace',
									rightTooltip:
										'If set, all users of this workspace will have access the to entire content of the object storage.'
								}}
							/>
							{#if storage.publicResource === true}
								<div class="pt-2"></div>
								<Alert
									type="warning"
									title="(Legacy) Object storage content and resource details are shared"
								>
									object public access is ON, which means that the entire content of the object
									store will be accessible to all the users of this workspace regardless of whether
									they have access the resource or not.
								</Alert>
							{/if}
						</div>
					{/if}
				{/if}
			</div>
		</svelte:fragment>
	</Popover>
{/snippet}

{#snippet advancedPermissionsEditor(rules: S3ResourceSettingsItem['advancedPermissions'])}
	<Alert title="Standard Unix-style glob syntax is supported">
		The following will be interpolated :
		<ul class="list-disc pl-6">
			<li><code>{'{username}'}</code> : Nickname of the user doing the request</li>
			<li><code>{'{group}'}</code> : Any group that the user belongs to</li>
			<li><code>{'{folder_read}'}</code> : Any folder that the user has read access to</li>
			<li><code>{'{folder_write}'}</code> : Any folder that the user has write access to</li>
		</ul>
		<br />
		Note that changes may take up to 1 minute to propagate due to cache invalidation
	</Alert>
	{#each rules ?? [] as item, idx}
		<div class="flex gap-2">
			<ClearableInput bind:value={item.pattern} placeholder="Pattern" />
			<MultiSelect
				items={[{ value: 'read' }, { value: 'write' }, { value: 'delete' }, { value: 'list' }]}
				bind:value={item.allow}
				disablePortal
				class="w-[20rem]"
				placeholder="Deny all access"
				hideMainClearBtn
			/>
			<CloseButton onClick={() => rules?.splice(idx, 1)} />
		</div>
	{/each}
	<Button size="xs" variant="default" on:click={() => rules?.push({ pattern: '', allow: [] })}>
		<Plus size={14} />
		Add permission rule
	</Button>
{/snippet}

<script lang="ts">
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { emptyString, pick, sendUserToast } from '$lib/utils'
	import { ChevronDown, Plus, Shield } from 'lucide-svelte'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import SettingsPageHeader from '../settings/SettingsPageHeader.svelte'
	import SettingsFooter from './SettingsFooter.svelte'
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
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Cell from '../table/Cell.svelte'
	import Row from '../table/Row.svelte'
	import { deepEqual } from 'fast-equals'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'

	let {
		s3ResourceSettings = $bindable(),
		s3ResourceSavedSettings,
		onSave = undefined,
		onDiscard = undefined
	}: {
		s3ResourceSettings: S3ResourceSettings
		s3ResourceSavedSettings: S3ResourceSettings
		onSave?: () => void
		onDiscard?: () => void
	} = $props()

	let advancedPermissionModalState:
		| { open: false }
		| { open: true; storage: S3ResourceSettingsItem } = $state({ open: false })

	let s3FileViewer: any | undefined = $state()

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
	let tableHeadNames = ['Name', 'Storage resource', '', ''] as const
	let tableHeadTooltips: Partial<Record<(typeof tableHeadNames)[number], string | undefined>> = {
		'Storage resource':
			'Which resource the workspace storage will point to. Note that all users of the workspace will be able to access the workspace storage regardless of the resource visibility.'
	}

	let tableRows: [string | null, S3ResourceSettingsItem][] = $derived([
		[null, s3ResourceSettings],
		...(s3ResourceSettings.secondaryStorage ?? [])
	])
	let secondaryStorageIsDirty: Record<string, boolean> = $derived(
		Object.fromEntries(
			s3ResourceSettings.secondaryStorage?.map((d) => {
				const saved = s3ResourceSavedSettings.secondaryStorage?.find((saved) => saved[0] === d[0])
				return [d[0], !deepEqual(saved?.[1], d[1])] as const
			}) ?? []
		)
	)

	let primaryStorageIsDirty: boolean = $derived.by(() => {
		const fields = [
			'resourcePath',
			'resourceType',
			'publicResource',
			'advancedPermissions'
		] as const
		return !deepEqual(pick(s3ResourceSavedSettings, fields), pick(s3ResourceSettings, fields))
	})
	function isDirty(name: string | null): boolean {
		return name === null ? primaryStorageIsDirty : secondaryStorageIsDirty[name]
	}

	function isPermissionsNonDefault(storage: S3ResourceSettingsItem): boolean {
		const defaultPerms = defaultS3AdvancedPermissions(!!$enterpriseLicense)
		return !deepEqual(storage.advancedPermissions, defaultPerms)
	}

	let hasUnsavedChanges = $derived.by(() => {
		return !deepEqual(s3ResourceSettings, s3ResourceSavedSettings)
	})
</script>

<Portal name="workspace-settings">
	<S3FilePicker bind:this={s3FileViewer} readOnlyMode={false} fromWorkspaceSettings={true} />
</Portal>

<SettingsPageHeader
	title="Workspace object storage (S3/Azure Blob/GCS)"
	description="Connect your Windmill workspace to your S3 bucket, Azure Blob storage, or Google Cloud Storage to enable users to read and write from object storage without having to have access to the credentials."
	link="https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill#workspace-object-storage"
/>
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
	<DataTable containerClass="storage-settings-table mt-4">
		<Head>
			<tr>
				{#each tableHeadNames as name, i}
					<Cell head first={i == 0} last={i == tableHeadNames.length - 1}>
						{name}
						{#if tableHeadTooltips[name]}
							<Tooltip>{@html tableHeadTooltips[name]}</Tooltip>
						{/if}
					</Cell>
				{/each}
			</tr>
		</Head>
		<tbody class="divide-y bg-surface-tertiary">
			{#each tableRows as tableRow, idx}
				<Row>
					<Cell first class="w-48 relative">
						{#if tableRow[0] === null}
							<TextInput inputProps={{ placeholder: 'Primary storage', disabled: true }} />
						{:else}
							<TextInput
								bind:value={tableRow[0]}
								inputProps={{ placeholder: 'Name' }}
								class="secondary-storage-name-input"
							/>
						{/if}
					</Cell>
					<Cell>
						<div class="flex gap-2">
							<div class="relative">
								<Select
									items={[
										{ value: 's3', label: 'S3' },
										{ value: 'azure_blob', label: 'Azure Blob' },
										{ value: 's3_aws_oidc', label: 'AWS OIDC' },
										{ value: 'azure_workload_identity', label: 'Azure Workload Identity' },
										{ value: 'gcloud_storage', label: 'Google Cloud Storage' }
									]}
									bind:value={tableRow[1].resourceType}
									id="storage-resource-type-select"
									class="w-40"
								/>
							</div>
							<div class="flex flex-1">
								<ResourcePicker
									class="flex-1"
									bind:value={tableRow[1].resourcePath}
									resourceType={tableRow[1].resourceType}
								/>
							</div>
						</div>
					</Cell>

					<Cell class="w-12">
						<div class="flex gap-2">
							<Button
								variant="default"
								btnClasses="px-2.5 relative"
								size="sm"
								onClick={() =>
									(advancedPermissionModalState = { open: true, storage: tableRow[1] })}
							>
								<Shield size={16} /> Permissions <ChevronDown size={14} />
								{#if isPermissionsNonDefault(tableRow[1])}
									<span class="absolute -top-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-accent"
									></span>
								{/if}
							</Button>
							{#if emptyString(tableRow[1].resourcePath) || isDirty(tableRow[0])}
								<Popover
									openOnHover
									contentClasses="p-2 text-xs text-secondary"
									class="cursor-not-allowed"
								>
									<svelte:fragment slot="trigger">
										<ExploreAssetButton asset={{ kind: 's3object', path: '' }} disabled />
									</svelte:fragment>
									<svelte:fragment slot="content">
										{#if emptyString(tableRow[1].resourcePath)}
											Please select a storage resource
										{:else if isDirty(tableRow[0])}
											Please save your changes
										{/if}
									</svelte:fragment>
								</Popover>
							{:else}
								<ExploreAssetButton
									asset={{ kind: 's3object', path: (tableRow[0] ?? '') + '/' }}
									s3FilePicker={s3FileViewer}
								/>
							{/if}
						</div>
					</Cell>
					<Cell class="w-12">
						{#if tableRow[0] !== null}
							<CloseButton
								small
								on:close={() => {
									if (s3ResourceSettings.secondaryStorage) {
										s3ResourceSettings.secondaryStorage.splice(idx - 1, 1)
										s3ResourceSettings.secondaryStorage = [...s3ResourceSettings.secondaryStorage]
									}
								}}
							/>
						{/if}
					</Cell>
				</Row>
			{/each}
			<Row class="!border-0">
				<Cell colspan={tableHeadNames.length} class="pt-0 pb-2">
					{#snippet addSecondaryStorageBtn()}
						<Button
							size="sm"
							btnClasses="max-w-fit"
							variant="default"
							disabled={!s3ResourceSettings.resourcePath}
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
							}}
						>
							<Plus /> Add secondary storage
							{#if s3ResourceSettings.resourcePath}
								<Tooltip>
									Secondary storage is a feature that allows you to read and write from storage that
									isn't your main storage by specifying it in the s3 object as "secondary_storage"
									with the name of it
								</Tooltip>
							{/if}
						</Button>
					{/snippet}
					<div class="flex justify-center w-full">
						{#if !s3ResourceSettings.resourcePath}
							<Popover
								class="cursor-not-allowed"
								openOnHover
								contentClasses="p-2 text-xs text-secondary"
							>
								{#snippet trigger()}
									{@render addSecondaryStorageBtn()}
								{/snippet}
								{#snippet content()}
									Setup a primary storage to use secondary storages
								{/snippet}
							</Popover>
						{:else}
							{@render addSecondaryStorageBtn()}
						{/if}
					</div>
				</Cell>
			</Row>
		</tbody>
	</DataTable>

	<SettingsFooter
		class="mt-5 mb-5"
		inline
		{hasUnsavedChanges}
		onSave={editWindmillLFSSettings}
		onDiscard={() => onDiscard?.()}
		saveLabel="Save storage settings"
	/>
{/if}

<Modal2
	target="#content"
	title={'Permission settings'}
	contentClasses="flex flex-col gap-3"
	fixedWidth="md"
	fixedHeight="lg"
	isOpen={advancedPermissionModalState.open}
>
	{#if advancedPermissionModalState.open}
		{@const storage = advancedPermissionModalState.storage}
		{#if !$enterpriseLicense}
			<Alert
				type={storage.advancedPermissions ? 'error' : 'info'}
				title="Advanced permission rules are an Enterprise feature"
			>
				Consider upgrading to Windmill EE to use advanced permission rules to control access to your
				object storage at a more granular level.</Alert
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
							S3 resource public access is ON, which means that the entire content of the S3 bucket
							will be accessible to all the users of this workspace regardless of whether they have
							access the resource or not. Similarly, certain Windmill SDK endpoints can be used in
							scripts to access the resource details, including public and private keys.
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
							object public access is ON, which means that the entire content of the object store
							will be accessible to all the users of this workspace regardless of whether they have
							access the resource or not.
						</Alert>
					{/if}
				</div>
			{/if}
		{/if}
	{/if}
</Modal2>

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

	<div class="flex-1 overflow-y-auto gap-3 flex flex-col">
		{#each rules ?? [] as item, idx}
			<div class="flex gap-2">
				<ClearableInput bind:value={item.pattern} placeholder="Pattern" />
				<MultiSelect
					items={[{ value: 'read' }, { value: 'write' }, { value: 'delete' }, { value: 'list' }]}
					bind:value={item.allow}
					class="w-[20rem]"
					placeholder="Deny all access"
					hideMainClearBtn
				/>
				<CloseButton onClick={() => rules?.splice(idx, 1)} />
			</div>
		{/each}
	</div>
	<Button size="xs" variant="default" on:click={() => rules?.push({ pattern: '', allow: [] })}>
		<Plus size={14} />
		Add permission rule
	</Button>
{/snippet}

<script lang="ts">
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { displaySize, emptyString, pick, sendUserToast } from '$lib/utils'
	import { ChevronDown, Plus, RefreshCw, Shield } from 'lucide-svelte'
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
	import { HelpersService, WorkspaceService, type GetStorageUsageResponse } from '$lib/gen'
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
		loadStorageUsage(true)
	}

	let storageUsage: GetStorageUsageResponse | undefined = $state()
	let storageUsageLoading = $state(false)
	// Set on failure so the auto-load $effect below doesn't hammer a persistently
	// failing endpoint; cleared only by an explicit user-triggered refresh.
	let storageUsageErrored = $state(false)

	async function loadStorageUsage(refresh: boolean = false): Promise<void> {
		storageUsageLoading = true
		if (refresh) storageUsageErrored = false
		try {
			storageUsage = await HelpersService.getStorageUsage({
				workspace: $workspaceStore!,
				refresh
			})
		} catch (e) {
			storageUsageErrored = true
			console.error('Failed to load storage usage', e)
		} finally {
			storageUsageLoading = false
		}
	}

	$effect(() => {
		if (
			primaryStorageSaved &&
			storageUsage === undefined &&
			!storageUsageLoading &&
			!storageUsageErrored
		) {
			loadStorageUsage()
		}
	})

	let usedFraction: number | undefined = $derived(
		storageUsage?.quota_bytes
			? Math.min(storageUsage.total_bytes / storageUsage.quota_bytes, 1)
			: undefined
	)
	let overQuota: boolean = $derived(
		storageUsage?.quota_bytes !== undefined && storageUsage.total_bytes >= storageUsage.quota_bytes
	)
	let quotaDisplay: string = $derived(displaySize(storageUsage?.quota_bytes) ?? '10 GiB')
	let tableHeadNames = ['Name', 'Storage resource', '', ''] as const
	let tableHeadTooltips: Partial<Record<(typeof tableHeadNames)[number], string | undefined>> = {
		'Storage resource':
			'Which resource the workspace storage will point to. Note that all users of the workspace will be able to access the workspace storage regardless of the resource visibility.'
	}

	// Primary storage exists once it has been saved with a resource. Until then the row is
	// hidden and the user is offered an "Add primary storage" button instead. Visibility is an
	// explicit toggle (driven by Add/Delete) so editing or clearing the resource picker doesn't
	// make the row vanish.
	let primaryStorageSaved: boolean = $derived(!emptyString(s3ResourceSavedSettings.resourcePath))
	let showPrimaryRow: boolean = $state(primaryStorageSaved)

	// The parent reassigns the whole settings object on load and on every discard (footer or the
	// page-level "discard all changes"). Re-sync the row to the saved state when that happens.
	let prevSettings = s3ResourceSettings
	$effect(() => {
		if (s3ResourceSettings !== prevSettings) {
			prevSettings = s3ResourceSettings
			showPrimaryRow = primaryStorageSaved
		}
	})

	function clearPrimaryStorage() {
		s3ResourceSettings.resourceType = 's3'
		s3ResourceSettings.resourcePath = undefined
		s3ResourceSettings.publicResource = undefined
		s3ResourceSettings.advancedPermissions = defaultS3AdvancedPermissions(!!$enterpriseLicense)
		showPrimaryRow = false
	}

	let tableRows: [string | null, S3ResourceSettingsItem][] = $derived([
		...(showPrimaryRow
			? ([[null, s3ResourceSettings]] as [string | null, S3ResourceSettingsItem][])
			: []),
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

	// A visible storage row without a selected resource can't be saved.
	let hasMissingResource = $derived(tableRows.some(([, item]) => emptyString(item.resourcePath)))
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
	<Alert type="info" title="Workspace storage is limited to {quotaDisplay} in Windmill CE">
		Total workspace storage is capped at {quotaDisplay} in the Community Edition: writes that would exceed
		the quota are rejected. The bucket browser will also not work for buckets containing more than 20
		files. Consider upgrading to Windmill EE for unlimited workspace storage.
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
{#if primaryStorageSaved}
	<div class="mt-4 flex flex-col gap-1.5 max-w-xl storage-usage-section">
		<div class="flex items-center gap-2">
			<span class="text-sm font-semibold">Storage usage</span>
			<Button
				variant="default"
				size="xs2"
				iconOnly
				startIcon={{ icon: RefreshCw }}
				loading={storageUsageLoading}
				onclick={() => loadStorageUsage(true)}
				title="Recount usage by listing the storage"
			/>
		</div>
		{#if storageUsage}
			{#if storageUsage.quota_bytes !== undefined && usedFraction !== undefined}
				<div class="h-2 w-full rounded-full bg-surface-secondary overflow-hidden border">
					<div
						class="h-full rounded-full transition-all {overQuota
							? 'bg-red-500'
							: usedFraction >= 0.8
								? 'bg-yellow-500'
								: 'bg-accent'}"
						style="width: {Math.max(usedFraction * 100, 1)}%"
					></div>
				</div>
				<span class="text-xs text-secondary">
					{displaySize(storageUsage.total_bytes)} of {quotaDisplay} used
					{#if storageUsage.storages.length > 1}
						({storageUsage.storages
							.map(
								(s) =>
									`${s.storage === '_default_' ? 'primary' : s.storage}: ${displaySize(s.bytes)}`
							)
							.join(', ')})
					{/if}
				</span>
				{#if overQuota}
					<Alert type="error" title="Workspace storage quota exceeded">
						Writes to workspace storage are rejected until usage drops below {quotaDisplay}. Delete
						files from workspace storage or upgrade to Windmill EE for unlimited storage.
					</Alert>
				{/if}
			{:else}
				<span class="text-xs text-secondary">
					{displaySize(storageUsage.total_bytes)} used
					{#if storageUsage.storages.length > 1}
						({storageUsage.storages
							.map(
								(s) =>
									`${s.storage === '_default_' ? 'primary' : s.storage}: ${displaySize(s.bytes)}`
							)
							.join(', ')})
					{/if}
				</span>
			{/if}
		{:else if storageUsageLoading}
			<span class="text-xs text-tertiary">Computing storage usage...</span>
		{/if}
	</div>
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
			{#each tableRows as tableRow}
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
								{#if tableRow[1].resourceType === 'filesystem'}
									<!-- Filesystem storage is deliberately absent from the creatable
									     types below: it is dev-only (set via the API), so the UI only
									     renders it read-only when already configured. -->
									<Select
										items={[{ value: 'filesystem', label: 'Filesystem' }]}
										value={'filesystem'}
										disabled
										id="storage-resource-type-select"
										class="w-40"
									/>
								{:else}
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
								{/if}
							</div>
							<div class="flex flex-1">
								{#if tableRow[1].resourceType === 'filesystem'}
									<TextInput
										class="flex-1"
										value={tableRow[1].resourcePath ?? ''}
										inputProps={{ disabled: true, placeholder: 'Filesystem path' }}
									/>
								{:else}
									<ResourcePicker
										class="flex-1"
										bind:value={tableRow[1].resourcePath}
										resourceType={tableRow[1].resourceType}
										error={emptyString(tableRow[1].resourcePath)}
									/>
								{/if}
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
									{#snippet trigger()}
										<ExploreAssetButton asset={{ kind: 's3object', path: '' }} disabled />
									{/snippet}
									{#snippet content()}
										{#if emptyString(tableRow[1].resourcePath)}
											Please select a storage resource
										{:else if isDirty(tableRow[0])}
											Please save your changes
										{/if}
									{/snippet}
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
										const realIdx = s3ResourceSettings.secondaryStorage.findIndex(
											(s) => s === tableRow
										)
										if (realIdx !== -1) {
											s3ResourceSettings.secondaryStorage.splice(realIdx, 1)
											s3ResourceSettings.secondaryStorage = [...s3ResourceSettings.secondaryStorage]
										}
									}
								}}
							/>
						{:else if (s3ResourceSettings.secondaryStorage?.length ?? 0) === 0}
							<CloseButton small on:close={clearPrimaryStorage} />
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
						{#if !showPrimaryRow}
							<Button
								size="sm"
								btnClasses="max-w-fit mt-2"
								variant="default"
								on:click={() => (showPrimaryRow = true)}
							>
								<Plus /> Add primary storage
							</Button>
						{:else if !s3ResourceSettings.resourcePath}
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
		disabled={hasMissingResource}
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

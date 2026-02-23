<script lang="ts">
	import { page } from '$app/stores'
	import AppConnect from '$lib/components/AppConnectDrawer.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Badge, Button, Skeleton, Tab } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import Required from '$lib/components/Required.svelte'
	import { resourceTypesStore } from '$lib/components/resourceTypesStore'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import SupabaseConnect from '$lib/components/SupabaseConnect.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { ResourceType, WorkspaceDeployUISettings } from '$lib/gen'
	import { OauthService, ResourceService, WorkspaceService, type ListableResource } from '$lib/gen'
	import {
		enterpriseLicense,
		userStore,
		workspaceStore,
		userWorkspaces,
		globalDbManagerDrawer
	} from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import {
		canWrite,
		classNames,
		debounce,
		emptySchema,
		removeMarkdown,
		truncate,
		validateFileExtension
	} from '$lib/utils'
	import { isDeployable, ALL_DEPLOYABLE } from '$lib/utils_deployable'

	import { convert } from '@redocly/json-to-json-schema'
	import {
		Boxes,
		Braces,
		Building,
		Circle,
		FileUp,
		Link,
		Pen,
		Plus,
		RotateCw,
		Save,
		Share,
		Trash
	} from 'lucide-svelte'
	import { onMount, untrack } from 'svelte'
	import autosize from '$lib/autosize'
	import EditableSchemaWrapper from '$lib/components/schema/EditableSchemaWrapper.svelte'
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import ExploreAssetButton, {
		assetCanBeExplored
	} from '../../../../lib/components/ExploreAssetButton.svelte'
	import NoDirectDeployAlert from '$lib/components/NoDirectDeployAlert.svelte'

	type ResourceW = ListableResource & { canWrite: boolean; marked?: string }
	type ResourceTypeW = ResourceType & { canWrite: boolean }

	let cacheResources: ResourceW[] | undefined = $state()
	let stateResources: ResourceW[] | undefined = $state()
	let resources: ResourceW[] | undefined = $state()
	let resourceTypes: ResourceTypeW[] | undefined = $state()

	let filteredItems: (ResourceW & { marked?: string })[] | undefined = $state(undefined) as
		| (ResourceW & { marked?: string })[]
		| undefined

	let resourceTypeViewer: Drawer | undefined = $state(undefined)
	let resourceTypeViewerObj = $state({
		rt: '',
		description: '',
		schema: emptySchema(),
		formatExtension: undefined as string | undefined,
		isFileset: false
	})

	let resourceTypeDrawer: Drawer | undefined = $state(undefined)
	let editResourceTypeDrawer: Drawer | undefined = $state(undefined)
	let newResourceType = $state({
		name: '',
		schema: emptySchema(),
		description: '',
		formatExtension: undefined as string | undefined,
		isFileset: undefined as boolean | undefined
	})
	let isNewResourceTypeNameValid: boolean = $state(false)
	let resourceTypeNameExists: boolean = $state(false)

	let editResourceType = $state({
		name: '',
		schema: emptySchema(),
		description: '',
		formatExtension: undefined as string | undefined,
		isFileset: false
	})
	let resourceEditor: ResourceEditorDrawer | undefined = $state(undefined)
	let shareModal: ShareModal | undefined = $state(undefined)
	let appConnect: AppConnect | undefined = $state(undefined)
	let supabaseConnect: SupabaseConnect | undefined = $state(undefined)
	let deleteConfirmedCallback: (() => void) | undefined = $state(undefined)
	let loading = $state({
		resources: true,
		types: true
	})

	let filter = $state('')
	let ownerFilter: string | undefined = $state(undefined)

	let showCreateButtons = $state(false)

	let typeFilter: string | undefined = $state(undefined)

	async function loadResources(): Promise<void> {
		resources = await loadResourceInternal(undefined, 'cache,state')
		loading.resources = false
	}

	async function loadCache(): Promise<void> {
		cacheResources = await loadResourceInternal('cache', undefined)
		loading.resources = false
	}

	async function loadState(): Promise<void> {
		stateResources = await loadResourceInternal('state', undefined)
		loading.resources = false
	}

	async function loadResourceInternal(
		resourceType: string | undefined,
		resourceTypeExclude: string | undefined
	): Promise<ResourceW[]> {
		return (
			await ResourceService.listResource({
				workspace: $workspaceStore!,
				resourceTypeExclude,
				resourceType
			})
		).map((x) => {
			return {
				canWrite:
					canWrite(x.path, x.extra_perms!, $userStore) && $workspaceStore! == x.workspace_id,
				...x
			}
		})
	}

	async function loadResourceTypes(): Promise<void> {
		resourceTypes = (await ResourceService.listResourceType({ workspace: $workspaceStore! })).map(
			(x) => {
				return {
					canWrite: $workspaceStore! == x.workspace_id,
					...x
				}
			}
		)
		loading.types = false
	}

	async function deleteResource(path: string, account?: number): Promise<void> {
		if (account) {
			OauthService.disconnectAccount({ workspace: $workspaceStore!, id: account })
		}
		await ResourceService.deleteResource({ workspace: $workspaceStore!, path })
		reload()
	}

	async function addResourceType(): Promise<void> {
		if (!newResourceType.isFileset) {
			if (newResourceType.formatExtension === '') {
				throw new Error('Invalid empty file extension (make sure it is selected)')
			}
			if (!validateFileExtension(newResourceType.formatExtension ?? 'txt')) {
				throw new Error('Invalid file extension')
			}
		}
		await ResourceService.createResourceType({
			workspace: $workspaceStore!,
			requestBody: {
				name: (disableCustomPrefix ? '' : 'c_') + newResourceType.name,
				schema: newResourceType.schema,
				description: newResourceType.description,
				format_extension: newResourceType.formatExtension,
				is_fileset: newResourceType.isFileset ?? undefined
			}
		})
		resourceTypeDrawer?.closeDrawer?.()
		sendUserToast('Resource type created')
		loadResourceTypes()
		$resourceTypesStore = undefined
	}

	async function updateResourceType(): Promise<void> {
		await ResourceService.updateResourceType({
			workspace: $workspaceStore!,
			path: editResourceType.name,
			requestBody: {
				schema: editResourceType.schema,
				description: editResourceType.description
			}
		})
		editResourceTypeDrawer?.closeDrawer?.()
		sendUserToast('Resource type updated')
		loadResourceTypes()
	}

	async function handleDeleteResourceType(name: string) {
		try {
			await ResourceService.deleteResourceType({ workspace: $workspaceStore!, path: name })
			loadResourceTypes()
		} catch (err) {
			if (err.status === 400 && err.body.includes('foreign key')) {
				sendUserToast(
					`Could not delete resource type because there are resources attached to it. Delete resources using this type first.`,
					true
				)
			} else {
				sendUserToast(`Could not delete resource type: ${err?.body || err}`, true)
			}
		}
	}

	const startNewType = () => {
		// Find a unique name by checking for duplicates and appending a number
		const baseName = 'my_resource_type'
		let uniqueName = baseName
		let counter = 2

		// Check if the base name or numbered variants exist
		while (true) {
			const fullName = (disableCustomPrefix ? '' : 'c_') + uniqueName
			const exists = resourceTypes?.some((rt) => rt.name === fullName) ?? false

			if (!exists) {
				break
			}

			uniqueName = `${baseName}_${counter}`
			counter++
		}

		newResourceType = {
			name: uniqueName,
			schema: emptySchema(),
			description: '',
			formatExtension: undefined,
			isFileset: undefined
		}
		validateResourceTypeName()

		resourceTypeDrawer?.openDrawer?.()
	}

	async function startEditResourceType(name: string) {
		const rt = await ResourceService.getResourceType({ workspace: $workspaceStore!, path: name })
		editResourceType = {
			name: rt.name,
			schema: rt.schema as any,
			description: rt.description ?? '',
			formatExtension: rt.format_extension,
			isFileset: rt.is_fileset ?? false
		}
		editResourceTypeDrawer?.openDrawer?.()
	}

	onMount(() => {
		const callback = $page.url.searchParams.get('callback')
		if (callback == 'supabase_wizard') {
			supabaseConnect?.open?.()
		}

		const connect_app = $page.url.searchParams.get('connect_app')
		if (connect_app) {
			const rt = connect_app ?? undefined
			if (rt == 'undefined') {
				appConnect?.open?.()
			} else {
				appConnect?.open?.(rt)
			}
		}
	})

	function openInferrer(): void {
		inferrer?.openDrawer?.()
	}

	let disableCustomPrefix = $state(false)
	let tab: 'workspace' | 'types' | 'states' | 'cache' | 'theme' = $state('workspace') as
		| 'workspace'
		| 'types'
		| 'states'
		| 'cache'
		| 'theme'

	let inferrer: Drawer | undefined = $state(undefined)
	let inferrerJson = $state('')

	function inferJson(): void {
		try {
			newResourceType.schema = {
				$schema: undefined,
				required: [],
				properties: {},
				...convert(JSON.parse(inferrerJson))
			}
		} catch (e) {
			sendUserToast(`Invalid JSON: ${e}`, true)
		}
		inferrer?.closeDrawer?.()
	}
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state(undefined)

	async function reload() {
		loading = {
			resources: true,
			types: true
		}
		if (tab == 'cache') {
			await loadCache()
		} else if (tab == 'states') {
			await loadState()
		} else {
			await loadResources()
		}
		await loadResourceTypes()
		loading = {
			resources: false,
			types: false
		}
	}

	let deployUiSettings: WorkspaceDeployUISettings | undefined = $state(undefined)

	async function getDeployUiSettings() {
		if (!$enterpriseLicense) {
			deployUiSettings = ALL_DEPLOYABLE
			return
		}
		let settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		deployUiSettings = settings.deploy_ui ?? ALL_DEPLOYABLE
	}
	getDeployUiSettings()

	function validateResourceTypeName() {
		const snakeCaseRegex = /^[a-z0-9]+(_[a-z0-9]+)*$/
		isNewResourceTypeNameValid = snakeCaseRegex.test(newResourceType.name)
		checkResourceTypeNameExists.debounced()
	}

	const checkResourceTypeNameExists = debounce(() => {
		if (!newResourceType.name) {
			resourceTypeNameExists = false
			return
		}
		const fullName = (disableCustomPrefix ? '' : 'c_') + newResourceType.name
		resourceTypeNameExists = resourceTypes?.some((rt) => rt.name === fullName) ?? false
	}, 300)

	// Re-check when prefix setting changes
	$effect(() => {
		if (disableCustomPrefix !== undefined) {
			checkResourceTypeNameExists.debounced()
		}
	})

	function toSnakeCase() {
		newResourceType.name = newResourceType.name
			.replace(/([a-z0-9])([A-Z])/g, '$1_$2')
			.replace(/[\W]+/g, '_')
			.replace(/_+/g, '_')
			.replace(/^_+|_+$/g, '')
			.toLowerCase()
		validateResourceTypeName()
	}

	let resourceNameToFileExtMap: Record<string, string> | undefined = undefined
	let resourceNameToIsFilesetMap: Record<string, boolean> | undefined = undefined

	let loadingResourceNameToFileExt = false
	async function loadResourceNameToFileExtMap() {
		if (resourceNameToFileExtMap != undefined) return
		if (loadingResourceNameToFileExt) {
			while (resourceNameToFileExtMap == undefined) {
				console.log('waiting for resourceNameToFileExtMap')
				await new Promise((resolve) => setTimeout(resolve, 100))
			}
			return
		}
		loadingResourceNameToFileExt = true
		try {
			const raw = (await ResourceService.fileResourceTypeToFileExtMap({
				workspace: $workspaceStore!
			})) as Record<string, string | { format_extension: string | null; is_fileset: boolean }>
			resourceNameToFileExtMap = {}
			resourceNameToIsFilesetMap = {}
			for (const [k, v] of Object.entries(raw)) {
				if (typeof v === 'string') {
					resourceNameToFileExtMap[k] = v
					resourceNameToIsFilesetMap![k] = false
				} else {
					if (v.format_extension) {
						resourceNameToFileExtMap[k] = v.format_extension
					}
					resourceNameToIsFilesetMap![k] = v.is_fileset ?? false
				}
			}
		} catch (e) {
			console.error('Error loading resourceNameToFileExtMap', e)
		} finally {
			loadingResourceNameToFileExt = false
		}
	}

	async function resourceNameToFileExt(resourceName: string) {
		await loadResourceNameToFileExtMap()
		return resourceNameToFileExtMap?.[resourceName]
	}

	async function resourceNameIsFileset(resourceName: string) {
		await loadResourceNameToFileExtMap()
		return resourceNameToIsFilesetMap?.[resourceName] ?? false
	}

	let owners = $derived(
		Array.from(
			new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
		).sort()
	)
	let types = $derived(Array.from(new Set(filteredItems?.map((x) => x.resource_type))).sort())
	$effect(() => {
		if ($workspaceStore) {
			ownerFilter = undefined
		}
	})
	let currentResources = $derived(
		tab == 'cache' ? cacheResources : tab == 'states' ? stateResources : resources
	)
	let preFilteredItemsOwners = $derived(
		ownerFilter == undefined
			? currentResources
			: currentResources?.filter((x) => x.path.startsWith(ownerFilter ?? ''))
	)
	let preFilteredType = $derived.by(() => {
		let l =
			typeFilter == undefined
				? preFilteredItemsOwners?.filter((x) => {
						return tab === 'workspace'
							? x.resource_type !== 'app_theme' &&
									x.resource_type !== 'state' &&
									x.resource_type !== 'cache'
							: tab === 'states'
								? x.resource_type === 'state'
								: tab === 'cache'
									? x.resource_type === 'cache'
									: tab === 'theme'
										? x.resource_type === 'app_theme'
										: true
					})
				: preFilteredItemsOwners?.filter((x) => {
						return (
							x.resource_type === typeFilter &&
							(tab === 'workspace'
								? x.resource_type !== 'app_theme' &&
									x.resource_type !== 'state' &&
									x.resource_type !== 'cache'
								: true)
						)
					})
		if (filterUserFolders) {
			l = l?.filter((item) => {
				if (filterUserFoldersType === 'only f/*') return item.path.startsWith('f/')
				if (filterUserFoldersType === 'u/username and f/*')
					return item.path.startsWith('f/') || item.path.startsWith(`u/${$userStore?.username}/`)
			})
		}
		return l
	})
	$effect(() => {
		if ($workspaceStore && $userStore) {
			untrack(() => {
				loadResources()
				loadResourceTypes()
			})
		}
	})

	onMount(() => {
		let hash = $page.url.hash
		if (hash.startsWith('#/resource/')) {
			console.log('hash', hash)
			let path = hash.slice(11)
			resourceEditor?.initEdit(path)
		}
	})

	let dbManagerDrawer = $derived(globalDbManagerDrawer.val) as any

	let filterUserFolders = $state(false)
	let filterUserFoldersType: 'only f/*' | 'u/username and f/*' | undefined = $derived(
		$userStore?.is_super_admin && $userStore.username.includes('@')
			? 'only f/*'
			: $userStore?.is_admin || $userStore?.is_super_admin
				? 'u/username and f/*'
				: undefined
	)
</script>

<ConfirmationModal
	open={Boolean(deleteConfirmedCallback)}
	title="Remove resource"
	confirmationText="Remove"
	on:canceled={() => {
		deleteConfirmedCallback = undefined
	}}
	on:confirmed={() => {
		if (deleteConfirmedCallback) {
			deleteConfirmedCallback()
		}
		deleteConfirmedCallback = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this resource?</span>
		<Alert type="info" title="Bypass confirmation">
			<div>
				You can press
				<Badge color="dark-gray">SHIFT</Badge>
				while removing a resource to bypass confirmation.
			</div>
		</Alert>
	</div>
</ConfirmationModal>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />

<Drawer bind:this={inferrer} size="800px">
	<DrawerContent title="Infer type from JSON" on:close={() => inferrer?.toggleDrawer?.()}>
		<SimpleEditor
			bind:code={inferrerJson}
			lang="json"
			class="h-full"
			fixedOverflowWidgets={false}
		/>
		{#snippet actions()}
			<Button size="sm" on:click={inferJson}>Infer</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<Drawer bind:this={resourceTypeViewer} size="800px">
	<DrawerContent title={resourceTypeViewerObj.rt} on:close={resourceTypeViewer.closeDrawer}>
		<div>
			<h1 class="mb-8 mt-4"
				><IconedResourceType
					name={resourceTypeViewerObj.rt}
					formatExtension={resourceTypeViewerObj.formatExtension}
					isFileset={resourceTypeViewerObj.isFileset}
				/></h1
			>
			{#if resourceTypeViewerObj.description}
				<div class="py-2 box prose mb-8 text-secondary">
					<GfmMarkdown md={resourceTypeViewerObj.description ?? ''} />
				</div>
			{/if}
			{#if resourceTypeViewerObj.isFileset}
				<Alert type="info" title="Fileset resource type">
					This resource type represents a collection of files. Each file is identified by its
					relative path and contains text content. In the CLI, filesets are stored as directories.
				</Alert>
			{:else if resourceTypeViewerObj.formatExtension}
				<Alert
					type="info"
					title="Plain text file resource (.{resourceTypeViewerObj.formatExtension})"
				>
					This resource type represents a plain text file with a <b
						>.{resourceTypeViewerObj.formatExtension}</b
					>
					extension. (e.g. <b>my_file.{resourceTypeViewerObj.formatExtension}</b>)
				</Alert>
			{:else}
				<SchemaViewer schema={resourceTypeViewerObj.schema} />
			{/if}
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={editResourceTypeDrawer} size="800px">
	<DrawerContent title="Edit resource type" on:close={editResourceTypeDrawer.closeDrawer}>
		{#snippet actions()}
			<Button
				startIcon={{ icon: Save }}
				on:click={updateResourceType}
				unifiedSize="md"
				variant="accent">Update</Button
			>
		{/snippet}
		<div class="flex flex-col gap-6">
			<label for="inp">
				<div class="mb-1 font-semibold text-emphasis text-xs gap-1 flex flex-row items-center"
					>Name
					<div class="flex flex-row items-center gap-x-4">
						<div class="flex flex-row items-center">
							<div class="inline-block">
								<input
									disabled
									id="inp"
									type="text"
									bind:value={editResourceType.name}
									class={classNames('!h-8  !border !border-gray-200')}
								/>
							</div>
						</div>
					</div>
				</div></label
			>
			<label>
				<div class="mb-1 font-semibold text-emphasis text-xs">Description</div>
				<textarea use:autosize autocomplete="off" bind:value={editResourceType.description}
				></textarea></label
			>
			<div>
				{#if editResourceType.isFileset}
					<Alert type="info" title="Fileset resource (.{editResourceType.formatExtension})">
						This resource type represents a collection of files. The schema cannot be edited.
					</Alert>
				{:else if editResourceType.formatExtension}
					<Alert type="info" title="Plain text file resource (.{editResourceType.formatExtension})">
						This resource type represents a plain text file with a <b
							>.{editResourceType.formatExtension}</b
						>
						extension. (e.g. <b>my_file.{editResourceType.formatExtension}</b>). The schema only
						contains a `content` field and thus cannot be edited.
					</Alert>
				{:else}
					<div class="mb-1 font-semibold text-emphasis text-xs">Schema</div>
					<div class="flex flex-col gap-2">
						<EditableSchemaWrapper bind:schema={editResourceType.schema} noPreview />
					</div>
				{/if}
			</div>
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={resourceTypeDrawer} size="1200px">
	<DrawerContent title="Create resource type" on:close={resourceTypeDrawer.closeDrawer}>
		{#snippet actions()}
			<Button
				unifiedSize="md"
				variant="accent"
				startIcon={{ icon: Save }}
				on:click={addResourceType}
				disabled={!isNewResourceTypeNameValid || resourceTypeNameExists}>Save</Button
			>
		{/snippet}
		<div class="flex flex-col gap-6">
			<label for="inp">
				<div class="mb-1 font-semibold text-emphasis text-xs gap-1 flex flex-row items-center"
					>Name<Required required={true} /><Tooltip>
						Resource types are synchronized with the official types on the hub regularly. The `c_`
						prefix is to avoid name clashes with them.
					</Tooltip></div
				>
				<div class="flex flex-row items-center gap-x-4">
					<div class="flex flex-row items-center">
						{#if !disableCustomPrefix}
							<span
								class="border border-r-0 rounded-l py-1 text-sm bg-surface-secondary inline-block w-8 h-8 px-2"
							>
								c_
							</span>
						{/if}

						<div class="inline-block">
							<input
								id="inp"
								type="text"
								bind:value={newResourceType.name}
								class={classNames('!h-8  !border ', !disableCustomPrefix ? '!rounded-l-none' : '')}
								oninput={validateResourceTypeName}
							/>
						</div>
					</div>
					{#if $userStore?.is_admin || $userStore?.is_super_admin}
						<Toggle
							bind:checked={disableCustomPrefix}
							options={{ right: 'disable c_ prefix (admin only)' }}
						/>
					{/if}
				</div>
				{#if newResourceType.name}
					{#if !isNewResourceTypeNameValid}
						<p class="mt-1 px-2 text-red-600 dark:text-red-400 text-2xs"
							>Name must be snake_case!
							<button onclick={toSnakeCase} class="text-blue-600">Fix...</button></p
						>
					{:else if resourceTypeNameExists}
						<p class="mt-1 px-2 text-red-600 dark:text-red-400 text-2xs"
							>A resource type with this name already exists!</p
						>
					{/if}
				{/if}
			</label>
			<label>
				<div class="mb-1 font-semibold text-emphasis text-xs">Description</div>
				<textarea use:autosize autocomplete="off" bind:value={newResourceType.description}
				></textarea></label
			>
			<div>
				<div class="flex justify-between w-full items-center mb-1">
					<div class="font-semibold text-emphasis text-xs">Schema</div>
					<div class="w-full flex flex-row-reverse">
						<Button
							on:click={openInferrer}
							unifiedSize="md"
							variant="default"
							startIcon={{ icon: Braces }}
						>
							Infer schema from a json value
						</Button>
					</div>
				</div>

				<div class="flex flex-col gap-2">
					<EditableSchemaWrapper
						bind:schema={newResourceType.schema}
						bind:formatExtension={newResourceType.formatExtension}
						bind:isFileset={newResourceType.isFileset}
						fullHeight
					/>
				</div>
			</div>
		</div>
	</DrawerContent>
</Drawer>

<SearchItems
	{filter}
	items={preFilteredType}
	bind:filteredItems
	f={(x) => x.path + ' ' + x.resource_type + ' ' + x.description + ' '}
/>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.resources}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title="Resources"
			tooltip="Save and permission rich objects (JSON) including credentials obtained through OAuth."
			documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
		>
			{#if showCreateButtons}
				<div class="flex flex-row justify-end gap-4">
					<Button
						variant="default"
						unifiedSize="md"
						startIcon={{ icon: Plus }}
						on:click={startNewType}
						aiId="resources-add-resource-type"
						aiDescription="Add resource type"
					>
						Add resource type
					</Button>
					<Button
						unifiedSize="md"
						variant="accent"
						startIcon={{ icon: Boxes }}
						on:click={() => appConnect?.open?.()}
						aiId="resources-add-resource"
						aiDescription="Add resource"
					>
						Add resource
					</Button>
				</div>
			{/if}
		</PageHeader>
		<NoDirectDeployAlert onUpdateCanEditStatus={(v) => (showCreateButtons = v)} />
		<div class="flex justify-between">
			<Tabs
				class="w-full"
				bind:selected={tab}
				on:selected={(e) => {
					if (e.detail == 'cache') {
						loading.resources = true
						loadCache()
					} else if (e.detail == 'states') {
						loading.resources = true
						loadState()
					}
				}}
			>
				<Tab value="workspace" label="Workspace" icon={Building} />
				<Tab value="types" label="Resource Types">
					{#snippet extra()}
						<Tooltip
							documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
						>
							Every resource has a Resource Type attached to it which contains its schema and make
							it easy in scripts and flows to accept only resources of a specific resource type.
						</Tooltip>
					{/snippet}
				</Tab>
				<Tab value="states" label="States">
					{#snippet extra()}
						<Tooltip>
							States are actually resources (but excluded from the Workspace tab for clarity).
							States are used by scripts to keep data persistent between runs of the same script by
							the same trigger (schedule or user)
						</Tooltip>
					{/snippet}
				</Tab>
				<Tab value="cache" label="Cache">
					{#snippet extra()}
						<Tooltip>
							Cached results are actually resources (but excluded from the Workspace tab for
							clarity). Cache are used by flows's step to cache result to avoid recomputing
							unnecessarily
						</Tooltip>
					{/snippet}
				</Tab>
				<Tab value="theme" label="Theme">
					{#snippet extra()}
						<Tooltip>
							Theme are actually resources (but excluded from the Workspace tab for clarity). Theme
							are used by the apps to customize their look and feel.
						</Tooltip>
					{/snippet}
				</Tab>
			</Tabs>
			<div class="flex">
				<Button
					variant="default"
					on:click={reload}
					startIcon={{
						icon: RotateCw,
						classes: loading.resources || loading.types ? 'animate-spin' : ''
					}}
				/>
			</div>
		</div>
		{#if tab == 'workspace' || tab == 'states' || tab == 'cache' || tab == 'theme'}
			<div class="pt-2">
				<input placeholder="Search Resource" bind:value={filter} class="input mt-1" />
			</div>
			<ListFilters bind:selectedFilter={ownerFilter} filters={owners} />
			{#if tab != 'states' && tab != 'cache'}
				<ListFilters
					queryName="app_filter"
					bind:selectedFilter={typeFilter}
					filters={types}
					resourceType
				/>
			{:else}
				<div class="h-4"></div>
			{/if}

			<div class="overflow-x-auto pb-40 mt-4"
				><div class="flex flex-row items-center justify-end gap-4 pb-2">
					{#if $userStore?.is_super_admin && $userStore.username.includes('@')}
						<Toggle size="xs" bind:checked={filterUserFolders} options={{ right: 'Only f/*' }} />
					{:else if $userStore?.is_admin || $userStore?.is_super_admin}
						<Toggle
							size="xs"
							bind:checked={filterUserFolders}
							options={{ right: `Only u/${$userStore.username} and f/*` }}
						/>
					{/if}
				</div>
				{#if loading.resources}
					<Skeleton layout={[0.5, [2], 1]} />
					{#each new Array(6) as _}
						<Skeleton layout={[[4], 0.7]} />
					{/each}
				{:else if filteredItems?.length == 0}
					<div class="flex flex-col items-center justify-center h-full">
						<div class="text-xs text-emphasis font-semibold">No resources found</div>
						<div class="text-2xs text-secondary font-normal">
							Try changing the filters or creating a new resource
						</div>
					</div>
				{:else}
					<DataTable>
						<Head>
							<Row>
								<Cell head first />
								<Cell head>Path</Cell>
								<Cell head>Resource type</Cell>
								<Cell head>Description</Cell>
								<Cell head />
								<Cell head last />
							</Row>
						</Head>
						<tbody class="divide-y bg-surface">
							{#if filteredItems}
								{#each filteredItems as { path, description, resource_type, extra_perms, canWrite, is_oauth, is_linked, account, refresh_error, is_expired, marked, is_refreshed }}
									<Row>
										<Cell first>
											<SharedBadge {canWrite} extraPerms={extra_perms} />
										</Cell>
										<Cell>
											<a
												class="break-all"
												href="#/resource/{path}"
												onclick={() => resourceEditor?.initEdit?.(path)}
												>{#if marked}{@html marked}{:else}{path}{/if}</a
											>
										</Cell>
										<Cell>
											<a
												href="#{name}"
												onclick={() => {
													const linkedRt = resourceTypes?.find((rt) => rt.name === resource_type)
													if (linkedRt) {
														resourceTypeViewerObj = {
															rt: linkedRt.name,
															//@ts-ignore
															schema: linkedRt.schema,
															description: linkedRt.description ?? '',
															formatExtension: linkedRt.format_extension,
															isFileset: linkedRt.is_fileset ?? false
														}
														resourceTypeViewer?.openDrawer?.()
													} else {
														sendUserToast(
															`Resource type ${resource_type} not found in workspace.`,
															true
														)
													}
												}}
											>
												<IconedResourceType
													name={resource_type}
													after={true}
													formatExtension={resourceNameToFileExt(resource_type)}
													isFileset={resourceNameIsFileset(resource_type)}
												/>
											</a>
										</Cell>
										<Cell>
											<span class="text-primary text-xs">
												{removeMarkdown(truncate(description ?? '', 30))}
											</span>
										</Cell>
										<Cell>
											<div class="flex flex-row text-center">
												<div class="w-10">
													{#if is_linked}
														<Popover>
															<Link size={16} />
															{#snippet text()}
																<div>
																	This resource is linked with a variable of the same path. They are
																	deleted and renamed together.
																</div>
															{/snippet}
														</Popover>
													{/if}
												</div>
												<div class="w-10">
													{#if is_refreshed}
														<Popover>
															<RotateCw />
															{#snippet text()}
																<div>
																	The OAuth token will be kept up-to-date in the background by
																	Windmill using its refresh token
																</div>
															{/snippet}
														</Popover>
													{/if}
												</div>

												{#if is_oauth}
													<div class="w-10 pt-1.5">
														{#if refresh_error}
															<Popover>
																<Circle
																	class="text-red-600 animate-[pulse_5s_linear_infinite] fill-current"
																	size={12}
																/>
																{#snippet text()}
																	<div>
																		Latest exchange of the refresh token did not succeed. Error: {refresh_error}
																	</div>
																{/snippet}
															</Popover>
														{:else if is_expired}
															<Popover>
																<Circle
																	class="text-yellow-600 animate-[pulse_5s_linear_infinite] fill-current"
																	size={12}
																/>

																{#snippet text()}
																	<div>
																		The access_token is expired, it will get renewed the next time
																		this variable is fetched or you can request is to be refreshed
																		in the dropdown on the right.
																	</div>
																{/snippet}
															</Popover>
														{:else}
															<Popover>
																<Circle
																	class="text-green-600 animate-[pulse_5s_linear_infinite] fill-current"
																	size={12}
																/>
																{#snippet text()}
																	<div>
																		The resource was connected through OAuth and the token is not
																		expired.
																	</div>
																{/snippet}
															</Popover>
														{/if}
													</div>
												{/if}
											</div>
										</Cell>
										<Cell class="flex justify-end">
											{#if path && assetCanBeExplored({ kind: 'resource', path }, { resource_type }) && !$userStore?.operator}
												<ExploreAssetButton
													asset={{ kind: 'resource', path }}
													{dbManagerDrawer}
													_resourceMetadata={{ resource_type }}
													class="w-24"
												/>
											{/if}
											<Dropdown
												class="w-fit"
												items={[
													{
														displayName: !canWrite ? 'View Permissions' : 'Share',
														icon: Share,
														action: () => {
															shareModal?.openDrawer?.(path, 'resource')
														}
													},
													{
														displayName: 'Edit',
														icon: Pen,
														disabled: !canWrite || !showCreateButtons,
														action: () => {
															resourceEditor?.initEdit?.(path)
														}
													},
													...(isDeployable('resource', path, deployUiSettings)
														? [
																{
																	displayName: 'Deploy to prod/staging',
																	icon: FileUp,
																	action: () => {
																		deploymentDrawer?.openDrawer(path, 'resource')
																	}
																}
															]
														: []),
													{
														displayName: 'Delete',
														disabled: !canWrite || !showCreateButtons,
														icon: Trash,
														type: 'delete',
														action: (event) => {
															// TODO
															// @ts-ignore
															if (event?.shiftKey) {
																deleteResource(path, account)
															} else {
																deleteConfirmedCallback = () => {
																	deleteResource(path, account)
																}
															}
														}
													},
													...(account != undefined
														? [
																{
																	displayName: 'Refresh token',
																	icon: RotateCw,
																	action: async () => {
																		await OauthService.refreshToken({
																			workspace: $workspaceStore ?? '',
																			id: account ?? 0,
																			requestBody: {
																				path
																			}
																		})
																		sendUserToast('Token refreshed')
																		loadResources()
																	}
																}
															]
														: [])
												]}
											/>
										</Cell>
									</Row>
								{/each}
							{/if}
						</tbody>
					</DataTable>
				{/if}
			</div>
		{:else if tab == 'types'}
			{#if loading.types}
				<Skeleton layout={[0.5, [2], 1]} />
				{#each new Array(6) as _}
					<Skeleton layout={[[4], 0.7]} />
				{/each}
			{:else}
				<div class="overflow-auto mt-4">
					<DataTable>
						<Head>
							<Row>
								<Cell head first>Name</Cell>
								<Cell head>Description</Cell>
								<Cell head last />
							</Row>
						</Head>
						<tbody class="divide-y bg-surface">
							{#if resourceTypes}
								{#each resourceTypes as { name, description, schema, canWrite, format_extension, is_fileset }}
									<Row>
										<Cell first>
											<a
												href="#{name}"
												onclick={() => {
													resourceTypeViewerObj = {
														rt: name,
														//@ts-ignore
														schema: schema,
														description: description ?? '',
														formatExtension: format_extension,
														isFileset: is_fileset ?? false
													}

													resourceTypeViewer?.openDrawer?.()
												}}
											>
												<IconedResourceType
													after={true}
													{name}
													formatExtension={format_extension}
													isFileset={is_fileset}
												/>
											</a>
										</Cell>
										<Cell>
											<span class="text-primary text-xs w-96 flex flex-wrap whitespace-pre-wrap">
												{removeMarkdown(truncate(description ?? '', 200))}
											</span>
										</Cell>
										<Cell last>
											{#if !canWrite}
												<Badge>
													Shared globally
													<Tooltip>
														This resource type is from the 'admins' workspace shared with all
														workspaces
													</Tooltip>
												</Badge>
											{:else if $userStore?.is_admin || $userStore?.is_super_admin}
												<div class="flex flex-row-reverse gap-2">
													<Button
														size="xs"
														variant="default"
														disabled={!showCreateButtons}
														btnClasses="border-0"
														startIcon={{ icon: Trash }}
														on:click={() => handleDeleteResourceType(name)}
														destructive
													>
														Delete
													</Button>
													<Button
														size="xs"
														color="light"
														disabled={!showCreateButtons}
														startIcon={{ icon: Pen }}
														on:click={() => startEditResourceType(name)}
													>
														Edit
													</Button>
												</div>
											{:else}
												<Badge>
													Non Editable
													<Tooltip>
														Since resource types are shared with the whole workspace, only admins
														can edit/delete them
													</Tooltip>
												</Badge>
											{/if}
										</Cell>
									</Row>
								{/each}
							{/if}
						</tbody>
					</DataTable>
				</div>
			{/if}
		{/if}
	</CenteredPage>
{/if}

<SupabaseConnect bind:this={supabaseConnect} on:refresh={loadResources} />
<AppConnect bind:this={appConnect} on:refresh={loadResources} />
<ResourceEditorDrawer bind:this={resourceEditor} on:refresh={loadResources} />

<ShareModal
	bind:this={shareModal}
	on:change={() => {
		loadResources()
	}}
/>

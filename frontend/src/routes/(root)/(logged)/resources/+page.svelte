<script lang="ts">
	import { page } from '$app/stores'
	import AppConnect from '$lib/components/AppConnect.svelte'
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
	import ResourceEditor from '$lib/components/ResourceEditor.svelte'
	import { resourceTypesStore } from '$lib/components/resourceTypesStore'
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import SupabaseConnect from '$lib/components/SupabaseConnect.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { ResourceType } from '$lib/gen'
	import { OauthService, ResourceService, type ListableResource } from '$lib/gen'
	import { oauthStore, userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { canWrite, classNames, emptySchema, removeMarkdown, truncate } from '$lib/utils'
	import { convert } from '@redocly/json-to-json-schema'
	import {
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
	import { onMount } from 'svelte'
	import autosize from 'svelte-autosize'
	import Portal from 'svelte-portal'

	type ResourceW = ListableResource & { canWrite: boolean }
	type ResourceTypeW = ResourceType & { canWrite: boolean }

	let resources: ResourceW[] | undefined
	let resourceTypes: ResourceTypeW[] | undefined

	let filteredItems: (ResourceW & { marked?: string })[] | undefined = undefined

	let resourceTypeViewer: Drawer
	let resourceTypeViewerObj = {
		rt: '',
		description: '',
		schema: emptySchema()
	}

	let resourceTypeDrawer: Drawer
	let editResourceTypeDrawer: Drawer
	let newResourceType = {
		name: '',
		schema: emptySchema(),
		description: ''
	}
	let editResourceType = {
		name: '',
		schema: emptySchema(),
		description: ''
	}
	let resourceEditor: ResourceEditor | undefined
	let shareModal: ShareModal
	let appConnect: AppConnect
	let supabaseConnect: SupabaseConnect
	let deleteConfirmedCallback: (() => void) | undefined = undefined
	let loading = {
		resources: true,
		types: true
	}

	$: owners = Array.from(
		new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	$: types = Array.from(new Set(filteredItems?.map((x) => x.resource_type))).sort()

	let filter = ''
	let ownerFilter: string | undefined = undefined

	$: if ($workspaceStore) {
		ownerFilter = undefined
	}

	let typeFilter: string | undefined = undefined

	$: preFilteredItemsOwners =
		ownerFilter == undefined
			? resources
			: resources?.filter((x) => x.path.startsWith(ownerFilter ?? ''))

	$: preFilteredType =
		typeFilter == undefined
			? preFilteredItemsOwners?.filter((x) =>
					tab === 'workspace'
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
			  )
			: preFilteredItemsOwners?.filter(
					(x) =>
						x.resource_type === typeFilter &&
						(tab === 'workspace'
							? x.resource_type !== 'app_theme' &&
							  x.resource_type !== 'state' &&
							  x.resource_type !== 'cache'
							: true)
			  )

	async function loadResources(): Promise<void> {
		resources = (
			await ResourceService.listResource({
				workspace: $workspaceStore!
			})
		).map((x) => {
			return {
				canWrite:
					canWrite(x.path, x.extra_perms!, $userStore) && $workspaceStore! == x.workspace_id,
				...x
			}
		})

		loading.resources = false
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
		loadResources()
	}

	async function addResourceType(): Promise<void> {
		await ResourceService.createResourceType({
			workspace: $workspaceStore!,
			requestBody: {
				name: (disableCustomPrefix ? '' : 'c_') + newResourceType.name,
				schema: newResourceType.schema,
				description: newResourceType.description
			}
		})
		resourceTypeDrawer.closeDrawer?.()
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
		editResourceTypeDrawer.closeDrawer?.()
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
		newResourceType = {
			name: 'my_resource_type',
			schema: emptySchema(),
			description: ''
		}
		resourceTypeDrawer.openDrawer?.()
	}

	async function startEditResourceType(name: string) {
		const rt = await ResourceService.getResourceType({ workspace: $workspaceStore!, path: name })
		editResourceType = {
			name: rt.name,
			schema: rt.schema,
			description: rt.description ?? ''
		}
		editResourceTypeDrawer.openDrawer?.()
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadResources()
			loadResourceTypes()
		}
	}

	onMount(() => {
		let resource_type = $page.url.searchParams.get('resource_type')
		if ($oauthStore && resource_type) {
			appConnect.openFromOauth?.(resource_type)
		}

		const callback = $page.url.searchParams.get('callback')
		if (callback == 'supabase_wizard') {
			supabaseConnect.open?.()
		}

		const connect_app = $page.url.searchParams.get('connect_app')
		if (connect_app) {
			const rt = connect_app ?? undefined
			if (rt == 'undefined') {
				appConnect.open?.()
			} else {
				appConnect.open?.(rt)
			}
		}
	})

	function openInferrer(): void {
		inferrer?.openDrawer?.()
	}

	let disableCustomPrefix = false
	let tab: 'workspace' | 'types' | 'states' | 'cache' | 'theme' = 'workspace'

	let inferrer: Drawer | undefined = undefined
	let inferrerJson = ''

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
	let deploymentDrawer: DeployWorkspaceDrawer
</script>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />

<Portal>
	<Drawer bind:this={inferrer} size="800px">
		<DrawerContent title="Infer type from JSON" on:close={() => inferrer?.toggleDrawer?.()}>
			<SimpleEditor
				bind:code={inferrerJson}
				lang="json"
				class="h-full"
				fixedOverflowWidgets={false}
			/>
			<svelte:fragment slot="actions">
				<Button size="sm" on:click={inferJson}>Infer</Button>
			</svelte:fragment>
		</DrawerContent>
	</Drawer>
</Portal>

<Drawer bind:this={resourceTypeViewer} size="800px">
	<DrawerContent title={resourceTypeViewerObj.rt} on:close={resourceTypeViewer.closeDrawer}>
		<div>
			<h1 class="mb-8 mt-4"><IconedResourceType name={resourceTypeViewerObj.rt} /></h1>
			<div class="py-2 box prose mb-8">
				{resourceTypeViewerObj.description ?? ''}
			</div>
			<SchemaViewer schema={resourceTypeViewerObj.schema} />
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={editResourceTypeDrawer} size="800px">
	<DrawerContent title="Edit resource type" on:close={editResourceTypeDrawer.closeDrawer}>
		<svelte:fragment slot="actions">
			<Button startIcon={{ icon: Save }} on:click={updateResourceType}>Update</Button>
		</svelte:fragment>
		<div class="flex flex-col gap-6">
			<label for="inp">
				<div class="mb-1 font-semibold text-secondary gap-1 flex flex-row items-center"
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
				<div class="mb-1 font-semibold text-secondary">Description</div>
				<textarea
					type="text"
					use:autosize
					autocomplete="off"
					bind:value={editResourceType.description}
				/></label
			>
			<div>
				<div class="mb-1 font-semibold text-secondary">Schema</div>
				<SchemaEditor bind:schema={editResourceType.schema} />
			</div>
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={resourceTypeDrawer} size="800px">
	<DrawerContent title="Create resource type" on:close={resourceTypeDrawer.closeDrawer}>
		<svelte:fragment slot="actions">
			<Button startIcon={{ icon: Save }} on:click={addResourceType}>Save</Button>
		</svelte:fragment>
		<div class="flex flex-col gap-6">
			<label for="inp">
				<div class="mb-1 font-semibold text-secondary gap-1 flex flex-row items-center"
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
			</label>
			<label>
				<div class="mb-1 font-semibold text-secondary">Description</div>
				<textarea
					type="text"
					use:autosize
					autocomplete="off"
					bind:value={newResourceType.description}
				/></label
			>
			<div>
				<div class="flex justify-between w-full items-center">
					<div class="mb-1 font-semibold text-secondary">Schema</div>
					<div class="mb-2 w-full flex flex-row-reverse">
						<Button
							on:click={openInferrer}
							size="sm"
							color="light"
							variant="border"
							startIcon={{ icon: Braces }}
						>
							Infer schema from a json value
						</Button>
					</div>
				</div>
				<SchemaEditor bind:schema={newResourceType.schema} />
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

<CenteredPage>
	<PageHeader
		title="Resources"
		tooltip="Save and permission rich objects (JSON) including credentials obtained through OAuth."
		documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
	>
		<div class="flex flex-row justify-end gap-4">
			<Button variant="border" size="md" startIcon={{ icon: Plus }} on:click={startNewType}>
				Add Resource Type
			</Button>
			<Button size="md" startIcon={{ icon: Link }} on:click={() => appConnect.open?.()}>
				Add Resource
			</Button>
		</div>
	</PageHeader>
	<div class="flex justify-between">
		<Tabs class="w-full" bind:selected={tab}>
			<Tab size="md" value="workspace">
				<div class="flex gap-2 items-center my-1">
					<Building size={18} />
					Workspace
				</div>
			</Tab>
			<Tab size="md" value="types">
				<div class="flex gap-2 items-center my-1">
					Resource Types
					<Tooltip
						documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
					>
						Every resources have Resource Types attached to them which contains its schema and make
						it easy in scripts and flows to accept only resources of a specific resource type
					</Tooltip>
				</div>
			</Tab>
			<Tab size="md" value="states">
				<div class="flex gap-2 items-center my-1">
					States
					<Tooltip>
						States are actually resources (but excluded from the Workspace tab for clarity). States
						are used by scripts to keep data persistent between runs of the same script by the same
						trigger (schedule or user)
					</Tooltip>
				</div>
			</Tab>
			<Tab size="md" value="cache">
				<div class="flex gap-2 items-center my-1">
					Cache
					<Tooltip>
						Cached results are actually resources (but excluded from the Workspace tab for clarity).
						Cache are used by flows's step to cache result to avoid recomputing unnecessarily
					</Tooltip>
				</div>
			</Tab>
			<Tab size="md" value="theme">
				<div class="flex gap-2 items-center my-1">
					Theme
					<Tooltip>
						Theme are actually resources (but excluded from the Workspace tab for clarity). Theme
						are used by the apps to customize their look and feel.
					</Tooltip>
				</div>
			</Tab>
		</Tabs>
		<div class="flex">
			<Button
				variant="border"
				color="light"
				on:click={async () => {
					loading = {
						resources: true,
						types: true
					}
					await loadResources()
					await loadResourceTypes()
					loading = {
						resources: false,
						types: false
					}
				}}
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
			<div class="h-4" />
		{/if}

		<div class="overflow-x-auto pb-40">
			{#if loading.resources}
				<Skeleton layout={[0.5, [2], 1]} />
				{#each new Array(6) as _}
					<Skeleton layout={[[4], 0.7]} />
				{/each}
			{:else}
				<TableCustom>
					<tr slot="header-row">
						<th />
						<th>path</th>
						<th>resource type</th>
						<th>description</th>
						<th />
						<th />
					</tr>
					<tbody slot="body">
						{#if filteredItems}
							{#each filteredItems as { path, description, resource_type, extra_perms, canWrite, is_oauth, is_linked, account, refresh_error, is_expired, marked, is_refreshed }}
								<tr>
									<td class="!px-0 text-center">
										<SharedBadge {canWrite} extraPerms={extra_perms} />
									</td>
									<td>
										<a
											class="break-all"
											href="#{path}"
											on:click={() => resourceEditor?.initEdit?.(path)}
											>{#if marked}{@html marked}{:else}{path}{/if}</a
										>
									</td>
									<td class="px-2">
										<a
											href="#{name}"
											on:click={() => {
												const linkedRt = resourceTypes?.find((rt) => rt.name === resource_type)
												if (linkedRt) {
													resourceTypeViewerObj = {
														rt: linkedRt.name,
														schema: linkedRt.schema,
														description: linkedRt.description ?? ''
													}
													resourceTypeViewer.openDrawer?.()
												} else {
													sendUserToast(
														`Resource type ${resource_type} not found in workspace.`,
														true
													)
												}
											}}
										>
											<IconedResourceType name={resource_type} after={true} />
										</a>
									</td>
									<td>
										<span class="text-tertiary text-xs">
											{removeMarkdown(truncate(description ?? '', 30))}
										</span>
									</td>
									<td class="text-center">
										<div class="flex flex-row">
											<div class="w-10">
												{#if is_linked}
													<Popover>
														<Link />
														<div slot="text">
															This resource is linked with a variable of the same path. They are
															deleted and renamed together.
														</div>
													</Popover>
												{/if}
											</div>
											<div class="w-10">
												{#if is_refreshed}
													<Popover>
														<RotateCw />
														<div slot="text">
															The OAuth token will be kept up-to-date in the background by Windmill
															using its refresh token
														</div>
													</Popover>
												{/if}
											</div>

											{#if is_oauth}
												<div class="w-10">
													{#if refresh_error}
														<Popover>
															<Circle
																class="text-red-600 animate-[pulse_5s_linear_infinite] fill-current"
																size={12}
															/>
															<div slot="text">
																Latest exchange of the refresh token did not succeed. Error: {refresh_error}
															</div>
														</Popover>
													{:else if is_expired}
														<Popover>
															<Circle
																class="text-yellow-600 animate-[pulse_5s_linear_infinite] fill-current"
																size={12}
															/>

															<div slot="text">
																The access_token is expired, it will get renewed the next time this
																variable is fetched or you can request is to be refreshed in the
																dropdown on the right.
															</div>
														</Popover>
													{:else}
														<Popover>
															<Circle
																class="text-green-600 animate-[pulse_5s_linear_infinite] fill-current"
																size={12}
															/>
															<div slot="text">
																The resource was connected through OAuth and the token is not
																expired.
															</div>
														</Popover>
													{/if}
												</div>
											{/if}
										</div>
									</td>
									<td>
										<Dropdown
											items={[
												{
													displayName: !canWrite ? 'View Permissions' : 'Share',
													icon: Share,
													action: () => {
														shareModal.openDrawer?.(path, 'resource')
													}
												},
												{
													displayName: 'Edit',
													icon: Pen,
													disabled: !canWrite,
													action: () => {
														resourceEditor?.initEdit?.(path)
													}
												},
												{
													displayName: 'Deploy to prod/staging',
													icon: FileUp,
													action: () => {
														deploymentDrawer.openDrawer(path, 'resource')
													}
												},
												{
													displayName: 'Delete',
													disabled: !canWrite,
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
									</td>
								</tr>
							{/each}
						{:else if resources}
							<tr> No resources to display</tr>
						{:else}
							<tr>Loading...</tr>
						{/if}
					</tbody>
				</TableCustom>
			{/if}
		</div>
	{:else if tab == 'types'}
		{#if loading.types}
			<Skeleton layout={[0.5, [2], 1]} />
			{#each new Array(6) as _}
				<Skeleton layout={[[4], 0.7]} />
			{/each}
		{:else}
			<div class="overflow-auto">
				<TableCustom>
					<tr slot="header-row">
						<th>name</th>
						<th>description</th>
						<th />
					</tr>
					<tbody slot="body">
						{#if resourceTypes}
							{#each resourceTypes as { name, description, schema, canWrite }}
								<tr>
									<td>
										<a
											href="#{name}"
											on:click={() => {
												resourceTypeViewerObj = {
													rt: name,
													schema: schema,
													description: description ?? ''
												}

												resourceTypeViewer.openDrawer?.()
											}}
										>
											<IconedResourceType after={true} {name} />
										</a>
									</td>
									<td>
										<span class="text-tertiary text-xs">
											{removeMarkdown(truncate(description ?? '', 200))}
										</span>
									</td>
									<td>
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
													size="sm"
													color="red"
													variant="border"
													startIcon={{ icon: Trash }}
													on:click={() => handleDeleteResourceType(name)}
												>
													Delete
												</Button>
												<Button
													size="sm"
													variant="border"
													color="dark"
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
													Since resource types are shared with the whole workspace, only admins can
													edit/delete them
												</Tooltip>
											</Badge>
										{/if}
									</td>
								</tr>
							{/each}
						{:else if resources}
							<tr> No resources types to display</tr>
						{:else}
							<tr>Loading...</tr>
						{/if}
					</tbody>
				</TableCustom>
			</div>
		{/if}
	{/if}
</CenteredPage>

<SupabaseConnect bind:this={supabaseConnect} on:refresh={loadResources} />
<AppConnect bind:this={appConnect} on:refresh={loadResources} />
<ResourceEditor bind:this={resourceEditor} on:refresh={loadResources} />

<ShareModal
	bind:this={shareModal}
	on:change={() => {
		loadResources()
	}}
/>

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

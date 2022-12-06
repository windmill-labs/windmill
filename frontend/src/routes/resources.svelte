<script context="module">
	export function load() {
		return {
			stuff: { title: 'Resources' }
		}
	}
</script>

<script lang="ts">
	import { canWrite, emptySchema, removeMarkdown, sendUserToast, truncate } from '$lib/utils'
	import { OauthService, ResourceService, type ListableResource } from '$lib/gen'
	import type { ResourceType } from '$lib/gen'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ResourceEditor from '$lib/components/ResourceEditor.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import { userStore, workspaceStore, oauthStore, superadmin } from '$lib/stores'
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import {
		faEdit,
		faPlus,
		faShare,
		faTrash,
		faCircle,
		faChain,
		faSave,
		faRefresh
	} from '@fortawesome/free-solid-svg-icons'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Icon from 'svelte-awesome'
	import Required from '$lib/components/Required.svelte'
	import AppConnect from '$lib/components/AppConnect.svelte'
	import { page } from '$app/stores'
	import { onMount } from 'svelte'
	import { Button, Alert, Badge, Skeleton, Tab } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { Building } from 'svelte-lucide'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import autosize from 'svelte-autosize'

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
	let newResourceType = {
		name: '',
		schema: emptySchema(),
		description: ''
	}
	let resourceEditor: ResourceEditor | undefined
	let shareModal: ShareModal
	let appConnect: AppConnect
	let deleteConfirmedCallback: (() => void) | undefined = undefined
	let loading = {
		resources: true,
		types: true
	}

	$: open = Boolean(deleteConfirmedCallback)

	$: owners = Array.from(
		new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	$: types = Array.from(new Set(filteredItems?.map((x) => x.resource_type))).sort()

	let filter = ''
	let ownerFilter: string | undefined = undefined
	let typeFilter: string | undefined = undefined

	$: preFilteredItemsOwners =
		ownerFilter == undefined
			? resources
			: resources?.filter((x) => x.path.startsWith(ownerFilter ?? ''))

	$: preFilteredType =
		typeFilter == undefined
			? preFilteredItemsOwners
			: preFilteredItemsOwners?.filter((x) => x.resource_type == typeFilter)

	async function loadResources(): Promise<void> {
		resources = (await ResourceService.listResource({ workspace: $workspaceStore! })).map((x) => {
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
		if ($page.url.searchParams.get('connect_app')) {
			const rt = $page.url.searchParams.get('connect_app') ?? undefined
			if (rt == 'undefined') {
				appConnect.open?.()
			} else {
				appConnect.open?.(rt)
			}
		}
	})

	let disableCustomPrefix = false
	let tab: 'workspace' | 'types' = 'workspace'
</script>

<Drawer bind:this={resourceTypeViewer} size="800px">
	<DrawerContent title={resourceTypeViewerObj.rt} on:close={resourceTypeViewer.closeDrawer}>
		<div>
			<h1 class="mb-8 mt-4"><IconedResourceType name={resourceTypeViewerObj.rt} /></h1>
			<div class="py-2 box prose mb-8">
				<SvelteMarkdown source={resourceTypeViewerObj.description} />
			</div>
			<SchemaViewer schema={resourceTypeViewerObj.schema} />
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:this={resourceTypeDrawer} size="800px">
	<DrawerContent title="Create resource type" on:close={resourceTypeDrawer.closeDrawer}>
		<svelte:fragment slot="actions">
			<Button startIcon={{ icon: faSave }} on:click={addResourceType}>Save</Button>
		</svelte:fragment>
		<div class="flex flex-col gap-6">
			<label for="inp">
				<div class="mb-1 font-semibold text-gray-700">Name<Required required={true} /></div>
				<div class="flex flex-row items-center gap-x-4">
					{#if !disableCustomPrefix}
						<span
							class="border border-gray-700 rounded p-1 -mr-6 text-sm bg-gray-200 inline-block w-8"
							>c_</span
						>
					{/if}

					<div class="inline-block">
						<input id="inp" type="text" bind:value={newResourceType.name} />
					</div>

					{#if $userStore?.is_admin || $superadmin}
						<Toggle
							bind:checked={disableCustomPrefix}
							options={{ right: 'disable c_ prefix (admin only)' }}
						/>
						<Tooltip
							>Resource types are synchronized with the official types on the hub regularly. The
							`c_` prefix is to avoid name clashes with them.</Tooltip
						>
					{/if}
				</div>
			</label>
			<label>
				<div class="mb-1 font-semibold text-gray-700">Description</div>
				<textarea
					type="text"
					use:autosize
					autocomplete="off"
					bind:value={newResourceType.description}
				/></label
			>
			<div>
				<div class="mb-1 font-semibold text-gray-700">Schema</div>
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
	>
		<div class="flex flex-row space-x-4">
			<Button variant="border" size="md" startIcon={{ icon: faPlus }} on:click={startNewType}
				>Add a resource type</Button
			>
			<Button size="md" startIcon={{ icon: faChain }} on:click={() => appConnect.open?.()}>
				Add a resource/API
			</Button>
		</div>
	</PageHeader>
	<Tabs bind:selected={tab}>
		<Tab size="md" value="workspace">
			<div class="flex gap-2 items-center my-1">
				<Building size="18px" />
				Workspace
			</div>
		</Tab>
		<Tab size="md" value="types">
			<div class="flex gap-2 items-center my-1">
				Resource Types <Tooltip
					>Every resources have Resource Types attached to them which contains its schema and make
					it easy in scripts and flows to accept only resources of a specific resource type</Tooltip
				>
			</div>
		</Tab>
	</Tabs>
	{#if tab == 'workspace'}
		<div class="pt-2">
			<input placeholder="Search Resource" bind:value={filter} class="input mt-1" />
		</div>
		<ListFilters bind:selectedFilter={ownerFilter} filters={owners} />
		<ListFilters bind:selectedFilter={typeFilter} filters={types} />

		<div class="overflow-x-auto pb-40">
			{#if loading.resources}
				<Skeleton layout={[0.5, [2], 1]} />
				{#each new Array(6) as _}
					<Skeleton layout={[[4], 0.7]} />
				{/each}
			{:else}
				<TableCustom>
					<tr slot="header-row">
						<th>path</th>
						<th />
						<th>resource type</th>
						<th>description</th>
						<th />
						<th />
					</tr>
					<tbody slot="body">
						{#if filteredItems}
							{#each filteredItems as { path, description, resource_type, extra_perms, canWrite, is_oauth, is_linked, account, refresh_error, is_expired, marked }}
								<tr>
									<td>
										<a
											class="break-words"
											href="#{path}"
											on:click={() => resourceEditor?.initEdit?.(path)}
											>{#if marked}{@html marked}{:else}{path}{/if}</a
										>
									</td>
									<td><SharedBadge {canWrite} extraPerms={extra_perms} /></td>
									<td class="px-2"
										><a
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
											}}><IconedResourceType name={resource_type} after={true} /></a
										></td
									>
									<td
										><span class="text-gray-500 text-xs"
											>{removeMarkdown(truncate(description ?? '', 30))}</span
										></td
									>
									<td class="text-center">
										<div class="flex flex-row">
											<div class="w-10">
												{#if is_linked}
													<Popover>
														<Icon data={faChain} />
														<div slot="text">
															This resource is linked with a variable of the same path. They are
															deleted and renamed together.
														</div>
													</Popover>
												{/if}
											</div>
											<div class="w-10">
												{#if account}
													<Popover>
														<Icon data={faRefresh} />
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
															<Icon
																class="text-red-600 animate-[pulse_5s_linear_infinite]"
																data={faCircle}
																scale={0.7}
																label="Error during exchange of the refresh token"
															/>
															<div slot="text">
																Latest exchange of the refresh token did not succeed. Error: {refresh_error}
															</div>
														</Popover>
													{:else if is_expired}
														<Popover>
															<Icon
																class="text-yellow-600 animate-[pulse_5s_linear_infinite]"
																data={faCircle}
																scale={0.7}
																label="Variable is expired"
															/>
															<div slot="text">
																The access_token is expired, it will get renewed the next time this
																variable is fetched or you can request is to be refreshed in the
																dropdown on the right.
															</div>
														</Popover>
													{:else}
														<Popover>
															<Icon
																class="text-green-600 animate-[pulse_5s_linear_infinite]"
																data={faCircle}
																scale={0.7}
																label="Variable is tied to an OAuth app"
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
											dropdownItems={[
												{
													displayName: 'Share',
													icon: faShare,
													disabled: !canWrite,
													action: () => {
														shareModal.openDrawer?.(path)
													}
												},
												{
													displayName: 'Edit',
													icon: faEdit,
													disabled: !canWrite,
													action: () => {
														resourceEditor?.initEdit?.(path)
													}
												},
												{
													displayName: 'Delete',
													disabled: !canWrite,
													icon: faTrash,
													type: 'delete',
													action: (event) => {
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
																icon: faRefresh,
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
											relative={true}
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
									<td
										><a
											href="#{name}"
											on:click={() => {
												resourceTypeViewerObj = {
													rt: name,
													schema: schema,
													description: description ?? ''
												}

												resourceTypeViewer.openDrawer?.()
											}}><IconedResourceType after={true} {name} /></a
										></td
									>
									<td
										><span class="text-gray-500 text-xs"
											>{removeMarkdown(truncate(description ?? '', 200))}</span
										></td
									>
									<td>
										{#if !canWrite}
											<Badge
												>Shared globally<Tooltip
													>This resource type is from the 'starter' workspace shared with all
													workspaces</Tooltip
												></Badge
											>
										{:else if $userStore?.is_admin || $superadmin}
											<Button
												size="sm"
												color="red"
												variant="border"
												startIcon={{ icon: faTrash }}
												on:click={() => handleDeleteResourceType(name)}
											>
												Delete
											</Button>
										{:else}
											<Badge
												>Non Editable <Tooltip
													>Since resource types are shared with the whole workspace, only admins can
													edit/delete them</Tooltip
												></Badge
											>
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

<AppConnect bind:this={appConnect} on:refresh={loadResources} />
<ResourceEditor bind:this={resourceEditor} on:refresh={loadResources} />

<ShareModal
	bind:this={shareModal}
	kind="resource"
	on:change={() => {
		loadResources()
	}}
/>

<ConfirmationModal
	{open}
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

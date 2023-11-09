<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Badge, Button, Skeleton, Tab, Tabs } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import TableSimple from '$lib/components/TableSimple.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import VariableEditor from '$lib/components/VariableEditor.svelte'
	import type { ContextualVariable, ListableVariable } from '$lib/gen'
	import { OauthService, VariableService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { canWrite, isOwner, truncate } from '$lib/utils'
	import {
		Plus,
		FileUp,
		Link,
		Pen,
		RefreshCw,
		Share,
		Trash,
		Building,
		DollarSign,
		EyeOff,
		Circle
	} from 'lucide-svelte'

	type ListableVariableW = ListableVariable & { canWrite: boolean }

	let filter = ''
	let variables: ListableVariableW[] | undefined = undefined
	let filteredItems: (ListableVariableW & { marked?: string })[] | undefined = undefined
	let contextualVariables: ContextualVariable[] = []
	let shareModal: ShareModal
	let variableEditor: VariableEditor
	let loading = {
		contextual: true
	}

	let deleteConfirmedCallback: (() => void) | undefined = undefined
	$: open = Boolean(deleteConfirmedCallback)

	$: owners = Array.from(
		new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	let ownerFilter: string | undefined = undefined

	$: if ($workspaceStore) {
		ownerFilter = undefined
	}

	$: preFilteredItems =
		ownerFilter == undefined
			? variables
			: variables?.filter((x) => x.path.startsWith(ownerFilter ?? ''))

	// If relative, the dropdown is positioned relative to its button
	async function loadVariables(): Promise<void> {
		variables = (await VariableService.listVariable({ workspace: $workspaceStore! })).map((x) => {
			return {
				canWrite: canWrite(x.path, x.extra_perms!, $userStore) && x.workspace_id == $workspaceStore,
				...x
			}
		})
	}

	async function loadContextualVariables(): Promise<void> {
		contextualVariables = await VariableService.listContextualVariables({
			workspace: $workspaceStore!
		})
		loading.contextual = false
	}

	async function deleteVariable(path: string, account?: number): Promise<void> {
		if (account) {
			OauthService.disconnectAccount({ workspace: $workspaceStore!, id: account })
		}
		await VariableService.deleteVariable({ workspace: $workspaceStore!, path })
		loadVariables()
		sendUserToast(`Variable ${path} was deleted`)
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadVariables()
			loadContextualVariables()
		}
	}
	let tab: 'workspace' | 'contextual' = 'workspace'

	let deploymentDrawer: DeployWorkspaceDrawer
</script>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => x.path + ' ' + x.description}
/>

<CenteredPage>
	<PageHeader
		title="Variables"
		tooltip="Save and permission strings to be reused in Scripts and Flows."
		documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets"
	>
		<div class="flex flex-row justify-end">
			<Button size="md" startIcon={{ icon: Plus }} on:click={() => variableEditor.initNew()}>
				New&nbsp;variable
			</Button>
		</div>
	</PageHeader>

	<VariableEditor bind:this={variableEditor} on:create={loadVariables} />
	<ShareModal
		bind:this={shareModal}
		on:change={() => {
			loadVariables()
		}}
	/>

	<Tabs bind:selected={tab}>
		<Tab size="md" value="workspace">
			<div class="flex gap-2 items-center my-1">
				<Building size={18} />
				Workspace
			</div>
		</Tab>
		<Tab size="md" value="contextual">
			<div class="flex gap-2 items-center my-1">
				<DollarSign size={18} />
				Contextual
				<Tooltip
					documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets#contextual-variables"
				>
					Contextual variables are passed as environment variables when running a script and depends
					on the execution context.
				</Tooltip>
			</div>
		</Tab>
	</Tabs>
	{#if tab == 'workspace'}
		<div class="pt-2">
			<input placeholder="Search Variable" bind:value={filter} class="input mt-1" />
		</div>
		<div class="min-h-[56px]">
			<ListFilters bind:selectedFilter={ownerFilter} filters={owners} />
		</div>
		<div class="relative overflow-x-auto pb-40 pr-4">
			{#if !filteredItems}
				<Skeleton layout={[0.5, [2], 1]} />
				{#each new Array(3) as _}
					<Skeleton layout={[[3.5], 0.5]} />
				{/each}
			{:else if filteredItems.length == 0}
				<div class="flex flex-col items-center justify-center h-full">
					<div class="text-md font-medium">No variables found</div>
					<div class="text-sm text-secondary">
						Try changing the filters or creating a new variable
					</div>
				</div>
			{:else}
				<DataTable size="xs">
					<Head>
						<tr>
							<Cell head first class="!px-0" />
							<Cell head>Path</Cell>
							<Cell head>Value</Cell>
							<Cell head>Description</Cell>
							<Cell head />
							<Cell head last />
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each filteredItems as { path, value, is_secret, description, extra_perms, canWrite, account, is_refreshed, is_expired, refresh_error, is_linked, marked }}
							<Row>
								<Cell class="!px-0 text-center w-12" first>
									<SharedBadge {canWrite} extraPerms={extra_perms} />
								</Cell>
								<Cell>
									<a
										class="break-all"
										id="edit-{path}"
										on:click={() => variableEditor.editVariable(path)}
										href="#{path}"
									>
										{#if marked}
											{@html marked}
										{:else}
											{path}
										{/if}
									</a>
								</Cell>
								<Cell>
									<span class="inline-flex flex-row items-center gap-2">
										<div class="text-sm break-words">
											{#if value}
												{truncate(value, 20)}
											{:else}
												&lowast;&lowast;&lowast;&lowast;
											{/if}
										</div>
										{#if is_secret}
											<Popover notClickable>
												<EyeOff size={12} />
												<span slot="text">This item is secret</span>
											</Popover>
										{/if}
									</span>
								</Cell>
								<Cell class="break-words">
									<span class="text-xs text-tertiary">{truncate(description ?? '', 50)} </span>
								</Cell>

								<Cell class="text-center">
									<div class="flex flex-row items-center gap-4">
										{#if is_linked}
											<Popover notClickable>
												<Link size={16} />
												<div slot="text">
													This variable is linked with a resource of the same path. They are deleted
													and renamed together.
												</div>
											</Popover>
										{/if}
										{#if account}
											<Popover notClickable>
												<RefreshCw size={16} />
												<div slot="text">
													This OAuth token will be kept up-to-date in the background by Windmill
													using its refresh token
												</div>
											</Popover>
										{/if}

										{#if is_refreshed}
											<div class="">
												{#if refresh_error}
													<Popover notClickable>
														<div class="relative inline-flex justify-center items-center w-4 h-4">
															<Circle
																class="text-red-600 animate-ping absolute z-50 w-4 h-4 fill-current"
																size={12}
															/>
															<Circle
																class="text-red-600 relative inline-flex fill-current "
																size={12}
															/>
														</div>

														<div slot="text">
															Latest exchange of the refresh token did not succeed. Error: {refresh_error}
														</div>
													</Popover>
												{:else if is_expired}
													<Popover notClickable>
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
													<Popover notClickable>
														<Circle
															class="text-green-600 animate-[pulse_5s_linear_infinite] fill-current"
															size={12}
														/>
														<div slot="text">
															The variable was connected through OAuth and the token is not expired.
														</div>
													</Popover>
												{/if}
											</div>
										{/if}
									</div>
								</Cell>
								<Cell last shouldStopPropagation>
									<Dropdown
										items={() => {
											let owner = isOwner(path, $userStore, $workspaceStore)
											return [
												{
													displayName: 'Edit',
													icon: Pen,
													action: () => variableEditor.editVariable(path),
													disabled: !canWrite
												},
												{
													displayName: 'Delete',
													icon: Trash,
													type: 'delete',
													action: (event) => {
														if (event['shiftKey']) {
															deleteVariable(path, account)
														} else {
															deleteConfirmedCallback = () => {
																deleteVariable(path, account)
															}
														}
													},
													disabled: !owner
												},
												{
													displayName: 'Deploy to prod/staging',
													icon: FileUp,
													action: () => {
														deploymentDrawer.openDrawer(path, 'variable')
													}
												},
												{
													displayName: owner ? 'Share' : 'See Permissions',
													action: () => {
														shareModal.openDrawer(path, 'variable')
													},
													icon: Share
												},
												...(account != undefined
													? [
															{
																displayName: 'Refresh token',
																icon: RefreshCw,
																action: async () => {
																	await OauthService.refreshToken({
																		workspace: $workspaceStore ?? '',
																		id: account ?? 0,
																		requestBody: {
																			path
																		}
																	})
																	sendUserToast('Token refreshed')
																	loadVariables()
																}
															}
													  ]
													: [])
											]
										}}
									/>
								</Cell>
							</Row>
						{/each}
					</tbody>
				</DataTable>
			{/if}
		</div>
	{:else if tab == 'contextual'}
		<div class="overflow-auto">
			{#if loading.contextual}
				<Skeleton layout={[0.5, [2], 1]} />
				{#each new Array(8) as _}
					<Skeleton layout={[[2.8], 0.5]} />
				{/each}
			{:else}
				<TableSimple
					headers={['Name', 'Example of value', 'Description']}
					data={contextualVariables}
					keys={['name', 'value', 'description']}
				/>
			{/if}
		</div>
	{/if}
</CenteredPage>

<ConfirmationModal
	{open}
	title="Remove variable"
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
		<span>Are you sure you want to remove this variable?</span>
		<Alert type="info" title="Bypass confirmation">
			<div>
				You can press
				<Badge color="dark-gray">SHIFT</Badge>
				while removing a variable to bypass confirmation.
			</div>
		</Alert>
	</div>
</ConfirmationModal>

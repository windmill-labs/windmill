<script context="module">
	export function load() {
		return {
			stuff: { title: 'Variables' }
		}
	}
</script>

<script lang="ts">
	import { canWrite, sendUserToast, truncate } from '$lib/utils'
	import { OauthService, VariableService } from '$lib/gen'
	import type { ListableVariable, ContextualVariable } from '$lib/gen'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import TableSimple from '$lib/components/TableSimple.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import VariableEditor from '$lib/components/VariableEditor.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Icon from 'svelte-awesome'
	import { faPlus, faCircle, faEyeSlash } from '@fortawesome/free-solid-svg-icons'
	import { Button } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert, Badge, Skeleton } from '$lib/components/common'
	import Popover from '$lib/components/Popover.svelte'

	type ListableVariableW = ListableVariable & { canWrite: boolean }

	let variables: ListableVariableW[] = []
	let contextualVariables: ContextualVariable[] = []
	let shareModal: ShareModal
	let variableEditor: VariableEditor
	let loading = {
		variables: true,
		contextual: true
	}

	let deleteConfirmedCallback: (() => void) | undefined = undefined
	$: open = Boolean(deleteConfirmedCallback)

	// If relative, the dropdown is positioned relative to its button
	async function loadVariables(): Promise<void> {
		variables = (await VariableService.listVariable({ workspace: $workspaceStore! })).map((x) => {
			return {
				canWrite: canWrite(x.path, x.extra_perms!, $userStore) && x.workspace_id == $workspaceStore,
				...x
			}
		})
		loading.variables = false
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
</script>

<CenteredPage>
	<PageHeader
		title="Variables"
		tooltip="Save and permission strings to be reused in Scripts and Flows."
	>
		<Button size="sm" startIcon={{ icon: faPlus }} on:click={() => variableEditor.initNew()}>
			New&nbsp;variable
		</Button>
	</PageHeader>

	<VariableEditor bind:this={variableEditor} on:create={loadVariables} />
	<div class="relative overflow-x-auto pb-40 pr-4">
		{#if loading.variables}
			<Skeleton layout={[0.5, [2], 1]} />
			{#each new Array(3) as _}
				<Skeleton layout={[[3.5], 0.5]} />
			{/each}
		{:else}
			<TableCustom>
				<tr slot="header-row">
					<th>path</th>

					<th>value</th>
					<th>description</th>
					<th>OAuth</th>
					<th />
				</tr>
				<tbody slot="body">
					{#each variables as { path, value, is_secret, description, extra_perms, canWrite, account, is_oauth }}
						<tr>
							<td
								><a
									class="break-words"
									id="edit-{path}"
									on:click={() => variableEditor.editVariable(path)}
									href="#{path}">{path}</a
								>
								<div><SharedBadge {canWrite} extraPerms={extra_perms} /></div>
							</td>
							<td>
								<span class="inline-flex flex-row">
									<span class="text-sm break-words">
										{truncate(value ?? '****', 20)}
									</span>
									{#if is_secret}
										<Popover>
											<Icon
												label="Secret"
												class="text-gray-700 mb-2 ml-2"
												data={faEyeSlash}
												scale={0.8}
											/>
											<span slot="text">This item is secret</span>
										</Popover>
									{/if}
								</span>
							</td>
							<td class="break-words"
								><span class="text-xs text-gray-500">{truncate(description ?? '', 50)}</span></td
							>

							<td class="text-center">
								{#if is_oauth}
									<Popover>
										<Icon
											class="text-green-600 animate-[pulse_5s_linear_infinite]"
											data={faCircle}
											scale={0.7}
											label="Variable is tied to an OAuth app"
										/>
										<div slot="text">
											The variable is tied to an OAuth app. The token is refreshed automatically if
											applicable.
										</div>
									</Popover>
								{/if}
							</td>
							<td
								><Dropdown
									dropdownItems={[
										{
											displayName: 'Edit',
											action: () => variableEditor.editVariable(path),
											disabled: !canWrite
										},
										{
											displayName: 'Delete',

											action: (event) => {
												if (event?.shiftKey) {
													deleteVariable(path, account)
												} else {
													deleteConfirmedCallback = () => {
														deleteVariable(path, account)
													}
												}
											},
											disabled: !canWrite
										},
										{
											displayName: 'Share',
											action: () => {
												shareModal.openDrawer(path)
											},
											disabled: !canWrite
										},
										...(account != undefined
											? [
													{
														displayName: 'Refresh token',
														action: async () => {
															await OauthService.refreshToken({
																workspace: $workspaceStore ?? '',
																id: account ?? 0,
																requestBody: {
																	path
																}
															})
															sendUserToast('Token refreshed')
														}
													}
											  ]
											: [])
									]}
									relative={true}
								/></td
							>
						</tr>
					{/each}
				</tbody>
			</TableCustom>
		{/if}
	</div>

	<ShareModal
		bind:this={shareModal}
		kind="variable"
		on:change={() => {
			loadVariables()
		}}
	/>

	<PageHeader
		title="Contextual Variables"
		tooltip="
			Contextual variables are utility variables passed to your environment when running a script
			and depends on the execution context. Variables cannot use a reseved variable name."
		primary={false}
	/>

	<div class="overflow-auto">
		<div class="my-5" />
		{#if loading.contextual}
			<Skeleton layout={[0.5, [2], 1]} />
			{#each new Array(8) as _}
				<Skeleton layout={[[2.8], 0.5]} />
			{/each}
		{:else}
			<TableSimple
				headers={['name', 'example of value', 'description']}
				data={contextualVariables}
				keys={['name', 'value', 'description']}
				twTextSize="text-sm"
			/>
		{/if}
	</div>
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

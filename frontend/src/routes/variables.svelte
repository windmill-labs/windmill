<script lang="ts">
	import { canWrite, sendUserToast } from '$lib/utils'
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
	import { faPlus, faCircle } from '@fortawesome/free-solid-svg-icons'

	type ListableVariableW = ListableVariable & { canWrite: boolean }

	let variables: ListableVariableW[] = []
	let contextualVariables: ContextualVariable[] = []

	let shareModal: ShareModal
	let variableEditor: VariableEditor

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
	}

	async function deleteVariable(path: string, account?: string): Promise<void> {
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
	<PageHeader title="Variables">
		<button
			class="default-button"
			on:click={() => {
				variableEditor.initNew()
			}}
		>
			<Icon class="text-white mb-1" data={faPlus} scale={0.9} /> &nbsp; New variable</button
		>
	</PageHeader>

	<VariableEditor bind:this={variableEditor} on:create={loadVariables} />
	<div class="relative">
		<TableCustom>
			<tr slot="header-row">
				<th>path</th>
				<th>value</th>
				<th>secret</th>
				<th>description</th>
				<th>OAuth</th>
				<th />
			</tr>
			<tbody slot="body">
				{#each variables as { path, value, is_secret, description, extra_perms, canWrite, account, is_oauth }}
					<tr>
						<td
							><a
								id="edit-{path}"
								style="cursor: pointer;"
								on:click={() => variableEditor.editVariable(path)}>{path}</a
							>
							<div><SharedBadge {canWrite} extraPerms={extra_perms} /></div>
						</td>
						<td>{value ?? '******'}</td>
						<td>{is_secret ? 'secret' : 'visible'}</td>
						<td>{description}</td>
						<td>
							{#if is_oauth}
								<Icon
									class="text-green-600"
									data={faCircle}
									scale={0.7}
									label="Variable is tied to an OAuth app"
								/>
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
										action: () => deleteVariable(path, account),
										disabled: !canWrite
									},
									{
										displayName: 'Share',
										action: () => {
											shareModal.openModal(path)
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
															id: account,
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
								relative={false}
							/></td
						>
					</tr>
				{/each}
			</tbody>
		</TableCustom>
	</div>

	<ShareModal
		bind:this={shareModal}
		kind="variable"
		on:change={() => {
			loadVariables()
		}}
	/>
	<div class="my-10" />

	<PageHeader
		title="Contextual Variables"
		tooltip="
			Contextual variables are utility variables passed to your environment when running a script
			and depends on the execution context. Variables cannot use a reseved variable name."
		primary={false}
	/>

	<div class="my-5" />
	<TableSimple
		headers={['name', 'example of value', 'description']}
		data={contextualVariables}
		keys={['name', 'value', 'description']}
		twTextSize="text-sm"
	/>
</CenteredPage>

<style>
</style>

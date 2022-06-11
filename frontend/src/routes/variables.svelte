<script lang="ts">
	import { canWrite, sendUserToast } from '../utils'
	import { VariableService } from '../gen'
	import type { ListableVariable, ContextualVariable } from '../gen'
	import Dropdown from './components/Dropdown.svelte'
	import PageHeader from './components/PageHeader.svelte'
	import TableSimple from './components/TableSimple.svelte'
	import TableCustom from './components/TableCustom.svelte'
	import ShareModal from './components/ShareModal.svelte'
	import SharedBadge from './components/SharedBadge.svelte'
	import VariableEditor from './components/VariableEditor.svelte'
	import { userStore, workspaceStore } from './../stores'
	import CenteredPage from './components/CenteredPage.svelte'
	import Icon from 'svelte-awesome'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'

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

	async function deleteVariable(path: string): Promise<void> {
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
				<th />
			</tr>
			<tbody slot="body">
				{#each variables as { path, value, is_secret, description, extra_perms, canWrite }}
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
										action: () => deleteVariable(path),
										disabled: !canWrite
									},
									{
										displayName: 'Share',
										action: () => {
											shareModal.openModal(path)
										},
										disabled: !canWrite
									}
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
			and depends on the execution context. Variables cannot use a reseved variable name. Most
			reserved variables values are context-dependent of the script they are running in."
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

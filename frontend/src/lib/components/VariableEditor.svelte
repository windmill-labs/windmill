<script lang="ts">
	import { VariableService, WorkspaceService } from '$lib/gen'
	import { createEventDispatcher, untrack } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import { Save } from 'lucide-svelte'
	import VariableForm from './VariableForm.svelte'
	import { resource } from 'runed'

	const dispatch = createEventDispatcher()

	let editPath: string | undefined = $state(undefined)
	let editSession = $state(0)

	let path: string = $state('')
	let variable: { value: string; is_secret: boolean; description: string } = $state({
		value: '',
		is_secret: true,
		description: ''
	})
	let labels: string[] | undefined = $state(undefined)
	let wsSpecific = $state(false)
	let pathError = $state('')

	let drawer: Drawer | undefined = $state()
	let form: VariableForm | undefined = $state()

	const deployTo = resource(
		() => $workspaceStore,
		async (ws) =>
			ws ? (await WorkspaceService.getDeployTo({ workspace: ws })).deploy_to : undefined
	)

	const variableResource = resource(
		() => ({ path: editPath, ws: $workspaceStore, session: editSession }),
		async ({ path, ws }) => {
			if (!path || !ws) return null
			return await VariableService.getVariable({ workspace: ws, path, decryptSecret: false })
		}
	)

	const MAX_VARIABLE_LENGTH = 10000
	let edit = $derived(editPath !== undefined)
	let initialPath = $derived(editPath ?? '')
	let can_write = $derived(
		variableResource.current
			? variableResource.current.workspace_id == $workspaceStore &&
					canWrite(editPath ?? '', variableResource.current.extra_perms ?? {}, $userStore)
			: true
	)
	let valid = $derived(variable.value.length <= MAX_VARIABLE_LENGTH)

	$effect(() => {
		const v = variableResource.current
		if (v) {
			untrack(() => {
				path = v.path
				variable = {
					value: v.value ?? '',
					is_secret: v.is_secret,
					description: v.description ?? ''
				}
				labels = v.labels ?? undefined
				wsSpecific = v.ws_specific ?? false
			})
		}
	})

	export function initNew(): void {
		editPath = undefined
		editSession++
		path = ''
		variable = { value: '', is_secret: true, description: '' }
		labels = undefined
		wsSpecific = false
		drawer?.openDrawer()
	}

	export function editVariable(edit_path: string): void {
		editPath = edit_path
		editSession++
		drawer?.openDrawer()
	}

	async function loadSecret(): Promise<void> {
		if (!editPath) return
		const getV = await VariableService.getVariable({
			workspace: $workspaceStore ?? '',
			path: editPath,
			decryptSecret: true
		})
		variable.value = getV.value ?? ''
		form?.setCode(variable.value)
	}

	async function createVariable(): Promise<void> {
		await VariableService.createVariable({
			workspace: $workspaceStore!,
			requestBody: {
				path,
				value: variable.value,
				is_secret: variable.is_secret,
				description: variable.description,
				labels,
				ws_specific: wsSpecific
			}
		})
		sendUserToast(`Created variable ${path}`)
		dispatch('create')
		drawer?.closeDrawer()
	}

	async function updateVariable(): Promise<void> {
		const getV = variableResource.current
		if (!getV || !editPath) return
		try {
			await VariableService.updateVariable({
				workspace: $workspaceStore!,
				path: editPath,
				requestBody: {
					path: getV.path != path ? path : undefined,
					value: variable.value == '' ? undefined : variable.value,
					is_secret: getV.is_secret != variable.is_secret ? variable.is_secret : undefined,
					description: getV.description != variable.description ? variable.description : undefined,
					labels,
					ws_specific: wsSpecific
				}
			})
			sendUserToast(`Updated variable ${editPath}`)
			dispatch('create')
			drawer?.closeDrawer()
		} catch (err) {
			sendUserToast(`Could not update variable: ${err.body}`, true)
		}
	}
</script>

<Drawer bind:this={drawer} size="900px">
	<DrawerContent
		title={edit ? `Update variable at ${initialPath}` : 'Add a variable'}
		on:close={drawer?.closeDrawer}
	>
		<div class="flex flex-col gap-8">
			{#if !can_write}
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			{/if}
			<VariableForm
				bind:this={form}
				bind:path
				bind:pathError
				bind:variable
				bind:labels
				bind:wsSpecific
				{initialPath}
				deployTo={deployTo.current}
				{can_write}
				{edit}
				onLoadSecret={loadSecret}
			/>
		</div>
		{#snippet actions()}
			<Button
				on:click={() => (edit ? updateVariable() : createVariable())}
				disabled={!can_write || !valid || pathError != ''}
				startIcon={{ icon: Save }}
				variant="accent"
				size="sm"
			>
				{edit ? 'Update' : 'Save'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

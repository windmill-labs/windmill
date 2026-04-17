<script lang="ts">
	import { VariableService, WorkspaceService } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import { Save } from 'lucide-svelte'
	import VariableForm from './VariableForm.svelte'

	const dispatch = createEventDispatcher()

	let path: string = $state('')

	let variable: {
		value: string
		is_secret: boolean
		description: string
	} = $state({
		value: '',
		is_secret: true,
		description: ''
	})
	let labels: string[] | undefined = $state(undefined)

	let drawer: Drawer | undefined = $state()
	let edit = $state(false)
	let initialPath: string = $state('')
	let pathError = $state('')
	let can_write = $state(true)
	let wsSpecific = $state(false)
	let deployTo: string | undefined = $state(undefined)
	let form: VariableForm | undefined = $state()

	WorkspaceService.getDeployTo({ workspace: $workspaceStore! }).then((x) => {
		deployTo = x.deploy_to
	})

	export function initNew(): void {
		variable = {
			value: '',
			is_secret: true,
			description: ''
		}
		edit = false
		initialPath = ''
		path = ''
		labels = undefined
		can_write = true
		wsSpecific = false
		drawer?.openDrawer()
	}

	export async function editVariable(edit_path: string): Promise<void> {
		edit = true
		const getV = await VariableService.getVariable({
			workspace: $workspaceStore ?? '',
			path: edit_path,
			decryptSecret: false
		})
		can_write =
			getV.workspace_id == $workspaceStore &&
			canWrite(edit_path, getV.extra_perms ?? {}, $userStore)

		variable = {
			value: getV.value ?? '',
			is_secret: getV.is_secret,
			description: getV.description ?? ''
		}
		labels = getV.labels ?? undefined
		wsSpecific = getV.ws_specific ?? false
		initialPath = edit_path
		path = edit_path
		drawer?.openDrawer()
	}

	async function loadSecret(): Promise<void> {
		const getV = await VariableService.getVariable({
			workspace: $workspaceStore ?? '',
			path: initialPath,
			decryptSecret: true
		})
		variable.value = getV.value ?? ''
		form?.setCode(variable.value)
	}

	const MAX_VARIABLE_LENGTH = 10000

	let valid = $derived(variable.value.length <= MAX_VARIABLE_LENGTH)

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
		try {
			const getV = await VariableService.getVariable({
				workspace: $workspaceStore ?? '',
				path: initialPath,
				decryptSecret: false
			})

			await VariableService.updateVariable({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					path: getV.path != path ? path : undefined,
					value: variable.value == '' ? undefined : variable.value,
					is_secret: getV.is_secret != variable.is_secret ? variable.is_secret : undefined,
					description: getV.description != variable.description ? variable.description : undefined,
					labels,
					ws_specific: wsSpecific
				}
			})
			sendUserToast(`Updated variable ${initialPath}`)
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
				{deployTo}
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

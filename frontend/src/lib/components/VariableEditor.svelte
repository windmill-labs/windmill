<script lang="ts">
	import { canWrite, sendUserToast } from '$lib/utils'
	import { VariableService } from '$lib/gen'
	import Path from './Path.svelte'
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import Required from './Required.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Button, ToggleButton, ToggleButtonGroup } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
	import autosize from 'svelte-autosize'
	import Toggle from './Toggle.svelte'
	import { faSave } from '@fortawesome/free-solid-svg-icons'
	import SimpleEditor from './SimpleEditor.svelte'

	const dispatch = createEventDispatcher()

	let path: string = ''

	let variable: {
		value: string
		is_secret: boolean
		description: string
	} = {
		value: '',
		is_secret: true,
		description: ''
	}
	let valid = true

	let drawer: Drawer
	let edit = false
	let initialPath: string
	let pathError = ''
	let can_write = true

	export function initNew(): void {
		variable = {
			value: '',
			is_secret: true,
			description: ''
		}
		edit = false
		initialPath = ''
		path = ''
		can_write = true
		drawer.openDrawer()
	}

	export async function editVariable(edit_path: string): Promise<void> {
		edit = true
		const getV = await VariableService.getVariable({
			workspace: $workspaceStore ?? '',
			path: edit_path,
			decryptSecret: false
		})
		can_write =
			getV.workspace_id == $workspaceStore && canWrite(path, getV.extra_perms ?? {}, $userStore)

		variable = {
			value: getV.value ?? '',
			is_secret: getV.is_secret,
			description: getV.description ?? ''
		}
		initialPath = edit_path
		path = edit_path
		drawer.openDrawer()
	}

	export async function loadVariable(path: string): Promise<void> {
		const getV = await VariableService.getVariable({
			workspace: $workspaceStore ?? '',
			path,
			decryptSecret: true
		})
		variable.value = getV.value ?? ''
		editor?.setCode(variable.value)
	}

	const MAX_VARIABLE_LENGTH = 10000

	$: valid = variable.value.length <= MAX_VARIABLE_LENGTH

	async function createVariable(): Promise<void> {
		await VariableService.createVariable({
			workspace: $workspaceStore!,
			requestBody: {
				path,
				value: variable.value,
				is_secret: variable.is_secret,
				description: variable.description
			}
		})
		sendUserToast(`Created variable ${path}`)
		dispatch('create')
		drawer.closeDrawer()
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
					description: getV.description != variable.description ? variable.description : undefined
				}
			})
			sendUserToast(`Updated variable ${initialPath}`)
			dispatch('create')
			drawer.closeDrawer()
		} catch (err) {
			sendUserToast(`Could not update variable: ${err.body}`, true)
		}
	}
	let editorKind: 'plain' | 'json' | 'yaml' = 'plain'
	let editor: SimpleEditor | undefined = undefined
</script>

<Drawer bind:this={drawer} size="900px">
	<DrawerContent
		title={edit ? `Update variable at ${initialPath}` : 'Add a variable'}
		on:close={drawer.closeDrawer}
	>
		<div class="flex flex-col gap-6">
			<div>
				<div>
					{#if !can_write}
						<div class="m-2">
							<Alert type="warning" title="Only read access"
								>You only have read access to this resource and cannot edit it</Alert
							>
						</div>
					{/if}
					<span class="font-semibold text-gray-700">Path</span>
					<Path
						disabled={!can_write}
						bind:error={pathError}
						bind:path
						{initialPath}
						namePlaceholder="my_variable"
						kind="variable"
					/>
				</div>
				<div class="mt-4">
					<Toggle bind:checked={variable.is_secret} options={{ right: 'Secret' }} />
					<div class="mb-2" />
					{#if variable.is_secret}
						<Alert type="warning" title="Audit log for each access">
							Every secret is encrypted at rest and in transit with a key specific to this
							workspace. In addition, any read of a secret variable generates an audit log whose
							operation name is: variables.decrypt_secret
						</Alert>
					{/if}
				</div>
			</div>

			<div>
				<div class="mb-1">
					<span class="font-semibold text-gray-700">Variable value</span>
					<span class="text-sm text-gray-500 mr-4"
						>({variable.value.length}/{MAX_VARIABLE_LENGTH} characters)</span
					>
					{#if edit && variable.is_secret}<Button
							variant="border"
							size="xs"
							on:click={() => loadVariable(initialPath)}
							>Load secret value<Tooltip>Will generate an audit log</Tooltip></Button
						>{/if}
				</div>
				<div class="flex flex-row">
					{#if editorKind == 'plain'}
						<textarea
							disabled={!can_write}
							rows="4"
							type="text"
							use:autosize
							bind:value={variable.value}
							placeholder="Update variable value"
						/>
					{:else if editorKind == 'json'}
						<div class="border rounded mb-4 w-full border-gray-700">
							<SimpleEditor
								bind:this={editor}
								autoHeight
								lang="json"
								bind:code={variable.value}
								fixedOverflowWidgets={false}
							/>
						</div>
					{:else if editorKind == 'yaml'}
						<div class="border rounded mb-4 w-full border-gray-700">
							<SimpleEditor
								bind:this={editor}
								autoHeight
								lang="yaml"
								bind:code={variable.value}
								fixedOverflowWidgets={false}
							/>
						</div>
					{/if}

					<ToggleButtonGroup col bind:selected={editorKind}>
						<ToggleButton light position="center" value="plain" size="xs">Plain</ToggleButton>
						<ToggleButton light position="center" value="json" size="xs">Json</ToggleButton>
						<ToggleButton light position="center" value="yaml" size="xs">YAML</ToggleButton>
					</ToggleButtonGroup>
				</div>
			</div>
			<!-- {/if} -->

			<div>
				<div class="mb-1 font-semibold text-gray-700">
					Description
					<Required required={false} />
				</div>
				<textarea
					rows="4"
					type="text"
					use:autosize
					bind:value={variable.description}
					placeholder="Used for X"
				/>
			</div>
		</div>
		<svelte:fragment slot="actions">
			<Button
				on:click={() => (edit ? updateVariable() : createVariable())}
				disabled={!can_write || !valid || pathError != ''}
				startIcon={{ icon: faSave }}
			>
				{edit ? 'Update' : 'Save'}
			</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>

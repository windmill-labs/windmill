<script lang="ts">
	import { sendUserToast } from '$lib/utils'
	import { VariableService } from '$lib/gen'
	import Path from './Path.svelte'
	import { createEventDispatcher } from 'svelte'
	import { workspaceStore } from '$lib/stores'
	import Required from './Required.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
	import autosize from 'svelte-autosize'
	import Toggle from './Toggle.svelte'

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

	export function initNew(): void {
		variable = {
			value: '',
			is_secret: true,
			description: ''
		}
		edit = false
		initialPath = ''
		path = ''
		drawer.openDrawer()
	}

	export async function editVariable(path: string): Promise<void> {
		edit = true
		const getV = await VariableService.getVariable({
			workspace: $workspaceStore ?? '',
			path,
			decryptSecret: false
		})
		variable = {
			value: getV.value ?? '',
			is_secret: getV.is_secret,
			description: getV.description ?? ''
		}
		initialPath = path
		drawer.openDrawer()
	}

	export async function loadVariable(path: string): Promise<void> {
		const getV = await VariableService.getVariable({
			workspace: $workspaceStore ?? '',
			path,
			decryptSecret: true
		})
		variable.value = getV.value ?? ''
	}

	const MAX_VARIABLE_LENGTH = 3000

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
		sendUserToast(`Successfully created variable ${path}`)
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
			sendUserToast(`Successfully updated variable at ${initialPath}`)
			dispatch('create')
			drawer.closeDrawer()
		} catch (err) {
			sendUserToast(`Could not update variable: ${err.body}`, true)
		}
	}
</script>

<Drawer bind:this={drawer}>
	<DrawerContent
		title={edit ? `Update variable at ${initialPath}` : 'Add a variable'}
		on:close={drawer.closeDrawer}
	>
		<div class="flex flex-col gap-6">
			<div>
				<div>
					<div class="mb-1 font-semibold text-gray-700">General</div>
					<Path
						bind:error={pathError}
						bind:path
						{initialPath}
						namePlaceholder="my_variable"
						kind="variable"
					/>
				</div>
				<div class="mt-4">
					<Toggle bind:checked={variable.is_secret} options={{ right: 'Secret' }} />
					{#if variable.is_secret}
						<Alert type="warning" title="Not visible after this">
							If the variable is a secret, you will not be able to read the value of it from the
							variable editor UI but only within scripts.
							<Tooltip>
								Within scripts, every read of the value create the audit log:
								'variables.decrypt_secret'
							</Tooltip>
						</Alert>
					{/if}
				</div>
			</div>
			<!-- 
			{#if variable.is_secret}
				<div class="mb-1 col-span-10">
					<Password
						bind:password={variable.value}
						placeholder={'******** (only fill to update value)'}
						label={`<span class="font-semibold text-gray-700">Secret value</span>
							<span class="text-sm text-gray-500">(${variable.value.length}/3000 characters)</span>`}
					/>
				</div>
			{:else} -->
			<div>
				<div class="mb-1">
					<span class="font-semibold text-gray-700">Variable value</span>
					<span class="text-sm text-gray-500 mr-4">({variable.value.length}/3000 characters)</span>
					{#if edit && variable.is_secret}<Button
							variant="border"
							size="xs"
							on:click={() => loadVariable(initialPath)}
							>Load secret value (generate audit log)</Button
						>{/if}
				</div>
				<textarea
					rows="4"
					type="text"
					use:autosize
					bind:value={variable.value}
					placeholder="Update variable value"
				/>
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
		<div slot="submission">
			<Button
				on:click={() => (edit ? updateVariable() : createVariable())}
				disabled={!valid || pathError != ''}
			>
				{edit ? 'Save' : 'Add'}
			</Button>
		</div>
	</DrawerContent>
</Drawer>

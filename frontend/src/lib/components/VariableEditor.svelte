<script lang="ts">
	import Password from './Password.svelte'
	import { sendUserToast } from '$lib/utils'
	import { VariableService } from '$lib/gen'
	import AutosizedTextarea from './AutosizedTextarea.svelte'
	import Path from './Path.svelte'
	import Modal from './Modal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { workspaceStore } from '$lib/stores'
	import Required from './Required.svelte'

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

	let modal: Modal
	let edit = false
	let initialPath: string

	export function initNew(): void {
		variable = {
			value: '',
			is_secret: true,
			description: ''
		}
		edit = false
		modal.openModal()
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
		modal.openModal()
	}

	const MAX_VARIABLE_LENGTH = 3000

	$: valid = variable.value.length < MAX_VARIABLE_LENGTH

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
		modal.closeModal()
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
			modal.closeModal()
		} catch (err) {
			sendUserToast(`Could not update variable: ${err.body}`, true)
		}
	}
</script>

<Modal z="z-50" bind:this={modal}>
	<div slot="title">
		{#if edit}
			Update variable at {initialPath}
		{:else}
			Add a variable
		{/if}
	</div>

	<div slot="content">
		<div class="text-gray-700 text-2xs mb-6">
			Variables have a globally unique name represented by their path. When passed to scripts, <pre
				class="inline text-red-700 bg-gray-50 rounded round-sm">/</pre>
			are converted to
			<pre class="inline text-red-700 bg-gray-50 rounded round-sm">_</pre>
		</div>

		<div class="flex flex-col gap-2 ">
			<div>
				<div class="text-gray-700 mb-0 pb-0">path</div>
				<Path bind:path {initialPath} namePlaceholder="my/variable" />
			</div>

			<label class="block pb-6">
				<input type="checkbox" bind:checked={variable.is_secret} />
				<span class="ml-2">Secret</span>
			</label>
			<div class="font-semibold text-gray-700 col-span-10 }">
				{#if variable.is_secret}
					<Password
						bind:password={variable.value}
						placeholder={'******** (only fill to update value)'}
						label={'variable (max 3000 characters)'}
					/>
				{:else}
					<span>variable (max 3000 characters)</span>
					<AutosizedTextarea bind:value={variable.value} minRows={5} />
				{/if}
			</div>

			<div class="flex flex-col w-full">
				<span class="text-gray-700">Description<Required required={false} /></span>
				<AutosizedTextarea bind:value={variable.description} placeholder={''} minRows={3} />
			</div>
		</div>
	</div>

	<button
		slot="submission"
		class={valid ? 'default-button' : 'default-button-disabled'}
		type="button"
		on:click={() => {
			if (edit) {
				updateVariable()
			} else {
				createVariable()
			}
		}}
		disabled={!valid}
	>
		{#if edit}
			Save
		{:else}
			Add a variable
		{/if}
	</button>
</Modal>

<script lang="ts">
	import { displayDate } from '$lib/utils.js'
	import { InputService, type CreateInput, type RunnableType } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores.js'
	import { Button } from '$lib/components/common'
	import { Save } from 'lucide-svelte'
	import { sendUserToast } from '$lib/utils.js'
	import { createEventDispatcher } from 'svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	const dispatch = createEventDispatcher()

	export let runnableId: string | undefined
	export let runnableType: RunnableType | undefined
	export let args: object
	export let disabled: boolean = false
	export let small: boolean | undefined = undefined
	export let showTooltip: boolean | undefined = undefined

	let savingInputs = false

	async function saveInput(args: object) {
		savingInputs = true

		const requestBody: CreateInput = {
			name: 'Saved ' + displayDate(new Date()),
			args: args as any
		}

		try {
			await InputService.createInput({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				requestBody
			})
		} catch (err) {
			console.error(err)
			sendUserToast(`Failed to save Input: ${err}`, true)
		}

		savingInputs = false
		dispatch('update')
	}
</script>

<Button
	on:click={() => saveInput(args)}
	{disabled}
	loading={savingInputs}
	startIcon={{ icon: Save }}
	variant={small ? 'subtle' : 'default'}
	unifiedSize="md"
>
	<span>{small ? 'Save inputs' : 'Save current input'}</span>
	{#if showTooltip}
		<Tooltip>Shared inputs are available to anyone with access to the script</Tooltip>
	{/if}
</Button>

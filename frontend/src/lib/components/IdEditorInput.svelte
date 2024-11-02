<script lang="ts">
	import { ArrowRight } from 'lucide-svelte'
	import { Button } from './common'
	import { createEventDispatcher } from 'svelte'
	import { forbiddenIds } from './flows/idUtils'
	import { slide } from 'svelte/transition'

	export let initialId: string
	export let reservedIds: string[] = []
	export let label: string = 'Component ID'
	export let value = initialId
	export let buttonText = ''
	export let btnClasses = '!p-1 !w-[34px] !ml-1'
	export let acceptUnderScores = false

	let error = ''
	const dispatch = createEventDispatcher()
	const regex = acceptUnderScores ? /^[a-zA-Z][a-zA-Z0-9_]*$/ : /^[a-zA-Z][a-zA-Z0-9]*$/

	$: validateId(value, reservedIds)

	function validateId(id: string, reservedIds: string[]) {
		if (id == initialId) {
			error = ''
			return
		}
		if (!regex.test(value)) {
			error = 'The ID must include only letters and numbers and start with a letter'
		} else if (forbiddenIds.includes(value)) {
			error = 'This ID is reserved'
		} else if (reservedIds.some((rid) => rid === value)) {
			error = 'This ID is already in use'
		} else {
			error = ''
		}
	}

	let inputDiv: HTMLInputElement | undefined = undefined

	$: inputDiv?.focus()
</script>

<label class="block text-primary">
	{#if label != ''}
		<div class="pb-1 text-sm text-secondary">{label}</div>
	{/if}
	<div class="flex w-full">
		<input
			bind:this={inputDiv}
			type="text"
			bind:value
			class="!w-auto grow"
			on:click|stopPropagation={() => {}}
			on:keydown|stopPropagation={({ key }) => {
				if (key === 'Enter' && error === '' && value !== initialId) {
					dispatch('save', value)
				} else if (key == 'Escape') {
					dispatch('close')
				}
			}}
			on:keypress|stopPropagation
		/>
		<Button
			size="xs"
			color="blue"
			buttonType="button"
			{btnClasses}
			aria-label="Save ID"
			disabled={error != '' || value === initialId}
			on:click={() => {
				dispatch('save', value)
			}}
		>
			{buttonText}<ArrowRight size={18} />
		</Button>
	</div>
	{#if error != ''}
		<div
			transition:slide|local={{ duration: 100 }}
			class="w-full text-sm text-red-600 whitespace-pre-wrap pt-1"
		>
			{error}
		</div>
	{/if}
</label>

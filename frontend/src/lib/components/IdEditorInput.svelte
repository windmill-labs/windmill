<script lang="ts">
	import { stopPropagation, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { ArrowRight } from 'lucide-svelte'
	import { Button } from './common'
	import { forbiddenIds } from './flows/idUtils'
	import { slide } from 'svelte/transition'

	interface Props {
		initialId: string
		reservedIds?: string[]
		reservedPrefixes?: string[]
		label?: string
		value?: any
		buttonText?: string
		btnClasses?: string
		acceptUnderScores?: boolean
		onSave: ({ oldId, newId }: { oldId: string; newId: string }) => void
		onClose?: () => void
	}

	let {
		initialId,
		reservedIds = [],
		reservedPrefixes = [],
		label = 'Component ID',
		value = $bindable(initialId),
		buttonText = '',
		btnClasses = '!p-1 !w-[34px] !ml-1',
		acceptUnderScores = false,
		onSave,
		onClose
	}: Props = $props()

	let error = $state('')
	const regex = acceptUnderScores ? /^[a-zA-Z][a-zA-Z0-9_]*$/ : /^[a-zA-Z][a-zA-Z0-9]*$/

	function validateId(id: string, reservedIds: string[], reservedPrefixes: string[]) {
		if (id == initialId) {
			error = ''
			return
		}
		if (!regex.test(value)) {
			error = 'The ID must include only letters and numbers and start with a letter'
		} else if (forbiddenIds.includes(value)) {
			error = 'This ID is reserved'
		} else if (reservedPrefixes.some((prefix) => value.startsWith(prefix))) {
			error = 'This ID uses a reserved prefix'
		} else if (reservedIds.some((rid) => rid === value)) {
			error = 'This ID is already in use'
		} else {
			error = ''
		}
	}

	let inputDiv: HTMLInputElement | undefined = $state(undefined)

	$effect(() => {
		validateId(value, reservedIds, reservedPrefixes)
	})
	$effect(() => {
		inputDiv?.focus()
	})
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
			onclick={stopPropagation(() => {})}
			onkeydown={(e) => {
				e.stopPropagation()
				let key = e.key
				if (key === 'Enter' && error === '' && value !== initialId) {
					onSave({ oldId: initialId, newId: value })
				} else if (key == 'Escape') {
					onClose?.()
				}
			}}
			onkeypress={stopPropagation(bubble('keypress'))}
		/>
		<Button
			size="xs"
			color="blue"
			buttonType="button"
			{btnClasses}
			aria-label="Save ID"
			disabled={error != '' || value === initialId}
			onclick={() => {
				onSave({ oldId: initialId, newId: value })
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

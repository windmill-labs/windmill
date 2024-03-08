<script lang="ts">
	import { Calendar } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { Popup } from '..'
	import type { Placement } from '@floating-ui/core'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'

	export let date: string | undefined
	export let label: string

	const dispatch = createEventDispatcher()
	let input: HTMLInputElement

	export let placement: Placement = 'top-end'
</script>

<Popup floatingConfig={{ placement: placement, strategy: 'absolute' }}>
	<svelte:fragment slot="button">
		<button
			title="Open calendar picker"
			class="absolute bottom-1 right-2 top-1 py-1 min-w-min !px-2 items-center text-primary bg-surface border rounded center-center hover:bg-surface-hover transition-all cursor-pointer"
			aria-label="Open calendar picker"
			on:click={() => {
				input?.focus()
			}}
		>
			<Calendar size={14} />
		</button>
	</svelte:fragment>

	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label class="block text-primary">
		<div class="pb-1 text-sm text-secondary">{label}</div>
		<div class="flex w-full">
			<DateTimeInput
				value={date}
				on:change={(e) => {
					date = e.detail
					dispatch('change', date)
				}}
			/>
		</div>
	</label>
</Popup>

<script lang="ts">
	import { Calendar } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import type { Placement } from '@floating-ui/core'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'

	export let date: string | undefined
	export let label: string
	export let useDropdown: boolean = false
	export let clearable: boolean = false
	export let target: string | HTMLElement | undefined = undefined

	const dispatch = createEventDispatcher()
	let input: HTMLInputElement | undefined

	export let placement: Placement = 'top-end'
</script>

<Popover
	floatingConfig={{ placement: placement, strategy: 'absolute' }}
	portal={target}
	contentClasses="p-4"
>
	<svelte:fragment slot="trigger">
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
	<svelte:fragment slot="content">
		<!-- svelte-ignore a11y-label-has-associated-control -->
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<label class="block text-primary">
			<div class="pb-1 text-sm text-secondary">{label}</div>
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div on:click|stopPropagation class="flex w-full">
				<DateTimeInput
					{clearable}
					{useDropdown}
					value={date}
					on:change={(e) => {
						date = e.detail
						dispatch('change', date)
					}}
					on:clear={() => {
						dispatch('clear')
					}}
				/>
			</div>
		</label>
	</svelte:fragment>
</Popover>

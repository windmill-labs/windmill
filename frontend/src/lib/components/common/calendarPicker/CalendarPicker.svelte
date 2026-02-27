<script lang="ts">
	import { Calendar } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import type { Placement } from '@floating-ui/core'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		date?: string | null | undefined
		label: string
		useDropdown?: boolean
		clearable?: boolean
		target?: string | HTMLElement | undefined
		placement?: Placement
		class?: string
		onchange?: (value: string) => void
		onclear?: () => void
	}

	let {
		date = $bindable(undefined),
		label,
		useDropdown = false,
		clearable = false,
		target = undefined,
		placement = 'top-end',
		class: className = undefined,
		onchange = undefined,
		onclear = undefined
	}: Props = $props()

	let input: HTMLInputElement | undefined
</script>

<Popover
	floatingConfig={{ placement: placement, strategy: 'absolute' }}
	portal={target}
	contentClasses="p-4"
>
	{#snippet trigger()}
		<button
			title="Open calendar picker"
			class={twMerge(
				'absolute bottom-1 right-2 top-1 py-1 min-w-min !px-2.5 items-center text-primary bg-surface-secondary rounded center-center hover:bg-surface-hover transition-all cursor-pointer',
				className
			)}
			aria-label="Open calendar picker"
			onclick={() => {
				input?.focus()
			}}
		>
			<Calendar size={14} />
		</button>
	{/snippet}
	{#snippet content()}
		<!-- svelte-ignore a11y_label_has_associated_control -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<label class="block text-primary">
			<div class="pb-1 text-sm text-secondary">{label}</div>
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div onclick={(e) => e.stopPropagation()} class="flex w-full">
				<DateTimeInput
					{clearable}
					{useDropdown}
					value={date}
					onchange={(e) => {
						date = e.detail
						if (date) onchange?.(date)
					}}
					onclear={() => {
						onclear?.()
					}}
				/>
			</div>
		</label>
	{/snippet}
</Popover>

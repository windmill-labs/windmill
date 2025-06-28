<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { createSync } from '@melt-ui/svelte'

	const dispatch = createEventDispatcher()

	import { createToggleGroup, melt } from '@melt-ui/svelte'
	interface Props {
		id?: string | null | undefined
		selected?: string | string[] | undefined
		noWFull?: boolean
		disabled?: boolean
		tabListClass?: string
		allowEmpty?: boolean
		class?: string
		children?: import('svelte').Snippet<[any]>
	}

	let {
		id = undefined,
		selected = $bindable(undefined),
		noWFull = false,
		disabled = false,
		tabListClass = '',
		allowEmpty = false,
		class: className = '',
		children
	}: Props = $props()

	const {
		elements: { root, item },
		options: { disabled: disabledOption },
		states
	} = createToggleGroup({
		type: 'single',
		onValueChange: ({ curr, next }) => {
			if (next === undefined && !allowEmpty) {
				return curr
			}
			if (curr !== next && curr !== undefined) {
				dispatch('selected', next)
			}
			return next
		}
	})

	$effect(() => {
		$disabledOption = disabled
	})

	const sync = createSync(states)
	$effect(() => {
		sync.value(selected, (v) => (selected = v))
	})
</script>

<div
	use:melt={$root}
	class={twMerge(
		`h-8 flex ${noWFull ? '' : 'w-full'} ${disabled ? 'disabled' : ''}`,
		className,
		'flex items-center data-[orientation="vertical"]:flex-col'
	)}
	aria-label="Toggle button group"
	{id}
>
	<div class={twMerge('flex bg-surface-secondary rounded-md p-0.5 gap-1 h-full ', tabListClass)}>
		{@render children?.({ item, disabled })}
	</div>
</div>

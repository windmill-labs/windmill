<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { createSync } from '@melt-ui/svelte'

	export let id: string | null | undefined = undefined
	export let selected: string | string[] | undefined = undefined
	export let noWFull: boolean = false
	export let disabled: boolean = false
	export let tabListClass: string = ''
	export let allowEmpty: boolean = false

	const dispatch = createEventDispatcher()

	import { createToggleGroup, melt } from '@melt-ui/svelte'

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

	$: $disabledOption = disabled

	const sync = createSync(states)
	$: sync.value(selected, (v) => (selected = v))
</script>

<div
	use:melt={$root}
	class={twMerge(
		`h-8 flex ${noWFull ? '' : 'w-full'} ${disabled ? 'disabled' : ''}`,
		$$props.class,
		'flex items-center data-[orientation="vertical"]:flex-col'
	)}
	aria-label="Toggle button group"
	{id}
>
	<div class={twMerge('flex bg-surface-secondary rounded-md p-0.5 gap-1 h-full ', tabListClass)}>
		<slot {item} {disabled} />
	</div>
</div>

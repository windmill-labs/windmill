<script lang="ts">
	import { createAccordion, melt } from '@melt-ui/svelte'
	import { untrack } from 'svelte'

	type AccordionStores = {
		// Each is a melt store that, when called with an item id, returns the
		// attributes/action descriptor consumable by `use:melt={...}`.
		trigger: any
		content: any
		item: any
	}

	interface Props {
		/** Currently-open section ids. Bindable. Multiple sections can be open at once. */
		value?: string[]
		disabled?: boolean
		class?: string
		items: import('svelte').Snippet<[{ stores: AccordionStores; isOpen: (id: string) => boolean }]>
	}

	let { value = $bindable([]), disabled = false, class: className = '', items }: Props = $props()

	const {
		elements: { root, trigger, content, item },
		helpers: { isSelected },
		states,
		options: { disabled: disabledOption }
	} = createAccordion<true>({
		multiple: true,
		defaultValue: untrack(() => value),
		disabled: untrack(() => disabled)
	})

	function arraysEqual(a: readonly string[], b: readonly string[]) {
		if (a.length !== b.length) return false
		for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false
		return true
	}

	// Bridge external `value` prop ↔ melt's `states.value` store with a content-based
	// equality guard to avoid infinite update loops caused by fresh array identity each
	// time we write back through `bind:value` on the parent.
	let bridging = false

	// External -> internal
	$effect(() => {
		const v = (value ?? []).slice()
		if (bridging) return
		bridging = true
		states.value.update((cur) => {
			const arr = (cur as string[] | undefined) ?? []
			return arraysEqual(arr, v) ? cur : v
		})
		bridging = false
	})

	// Internal -> external
	$effect(() => {
		const unsub = states.value.subscribe((cur) => {
			if (bridging) return
			const arr = ((cur as string[] | undefined) ?? []).slice()
			if (arraysEqual(arr, value ?? [])) return
			bridging = true
			value = arr
			bridging = false
		})
		return unsub
	})

	$effect(() => {
		$disabledOption = disabled
	})
</script>

<div use:melt={$root} class={className}>
	{@render items({
		stores: { trigger: $trigger, content: $content, item: $item },
		isOpen: (id) => $isSelected(id)
	})}
</div>

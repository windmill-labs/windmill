<script lang="ts">
	import { getModifierKey } from '$lib/utils'

	import { ArrowLeft, ArrowRight, Copy, Scissors, Trash } from 'lucide-svelte'
	import ContextMenu from '$lib/components/ContextMenu.svelte'
	import ComponentCallbacks from './component/ComponentCallbacks.svelte'

	let componentCallbacks: ComponentCallbacks | undefined = undefined
</script>

<ComponentCallbacks bind:this={componentCallbacks} />
<ContextMenu
	contextMenu={{
		menuItems: [
			{
				label: 'Cut',
				onClick: () => {
					componentCallbacks?.handleCut(new KeyboardEvent('keydown'))
				},
				icon: Scissors,
				shortcut: `${getModifierKey()} + X`
			},
			{
				label: 'Copy',
				onClick: () => {
					componentCallbacks?.handleCopy(new KeyboardEvent('keydown'))
				},
				icon: Copy,
				shortcut: `${getModifierKey()} + C`
			},
			{
				label: 'Next',
				onClick: () => {
					componentCallbacks?.right(new KeyboardEvent('keydown'))
				},
				shortcut: `Right arrow`,
				icon: ArrowRight
			},
			{
				label: 'Previous',
				onClick: () => {
					componentCallbacks?.left(new KeyboardEvent('keydown'))
				},
				shortcut: `Left arrow`,
				icon: ArrowLeft
			},

			{
				label: 'Delete',
				onClick: () => {},
				icon: Trash,
				shortcut: `${getModifierKey()} + Del`,
				color: 'red'
			}
		]
	}}
>
	<slot />
</ContextMenu>

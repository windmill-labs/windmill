<script lang="ts">
	import { getModifierKey } from '$lib/utils'

	import { Copy, Scissors, Trash } from 'lucide-svelte'
	import ContextMenu from '$lib/components/ContextMenu.svelte'
	import ComponentCallbacks from './component/ComponentCallbacks.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import DeleteComponent from './settingsPanel/DeleteComponent.svelte'
	import { findComponentSettings } from './appUtils'

	export let id: string
	let componentCallbacks: ComponentCallbacks | undefined = undefined

	const { selectedComponent, app } = getContext<AppViewerContext>('AppViewerContext')
	const { movingcomponents } = getContext<AppEditorContext>('AppEditorContext')

	let deleteComponent: DeleteComponent | undefined = undefined
	$: componentSettings = $selectedComponent?.map((sc) => findComponentSettings($app, sc))
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
				shortcut: `${getModifierKey()} + X`,
				disabled: $movingcomponents?.includes($selectedComponent?.[0] ?? '')
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
				label: 'Delete',
				onClick: () => {
					deleteComponent?.removeGridElement()
				},
				icon: Trash,
				shortcut: `${getModifierKey()} + Del`,
				color: 'red'
			}
		]
	}}
	{id}
>
	<slot />
</ContextMenu>

<DeleteComponent {componentSettings} bind:this={deleteComponent} />

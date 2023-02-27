<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, GridItem } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined

	let jsonViewer: Drawer
</script>

{#if jsonViewer?.isOpen()}
	<div class="relative">
		<Drawer
			bind:this={jsonViewer}
			size="800px"
			customPosition={{ drawer: '!top-0 z-0 ', overlay: 'top-0', placement: 'top' }}
		>
			<DrawerContent title="Argument Details" on:close={jsonViewer.toggleDrawer} />
		</Drawer>
	</div>
{/if}

<button on:click={jsonViewer.toggleDrawer}>Open drawer</button>

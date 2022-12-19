<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import { faSave } from '@fortawesome/free-solid-svg-icons'
	import type { InlineScript } from '../../types'

	let scriptEditorDrawer: Drawer
	export let inlineScript: InlineScript

	export function openDrawer() {
		scriptEditorDrawer.openDrawer?.()
	}
</script>

<Drawer bind:this={scriptEditorDrawer} size="1200px">
	<DrawerContent
		title="Script Editor"
		noPadding
		forceOverflowVisible
		on:close={scriptEditorDrawer.closeDrawer}
	>
		{#if inlineScript}
			<ScriptEditor
				lang={inlineScript.language}
				bind:code={inlineScript.content}
				path={inlineScript.path}
				bind:schema={inlineScript.schema}
				fixedOverflowWidgets={false}
			/>
		{/if}
		<svelte:fragment slot="actions">
			<Button startIcon={{ icon: faSave }} disabled>Automatically Saved</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>

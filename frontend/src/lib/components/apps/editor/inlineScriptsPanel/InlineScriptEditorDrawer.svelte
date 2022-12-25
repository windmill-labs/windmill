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
				noSyncFromGithub
				lang={inlineScript.language}
				path={inlineScript.path}
				fixedOverflowWidgets={false}
				bind:code={inlineScript.content}
				bind:schema={inlineScript.schema}
			/>
		{/if}
		<svelte:fragment slot="actions">
			<Button startIcon={{ icon: faSave }} disabled>Automatically Saved</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>

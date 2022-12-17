<script lang="ts">
	import type { Schema } from '$lib/common'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import type { Preview } from '$lib/gen'
	import { faSave } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'

	type InlineScript = {
		content: string
		language: Preview.language
		path: string
		schema: Schema
	}

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	let scriptEditorDrawer: Drawer
	let selectedScript: InlineScript | undefined

	export function openDrawer(path: string) {
		if ($app.inlineScripts[path]) {
			selectedScript = $app.inlineScripts[path]
			scriptEditorDrawer.openDrawer?.()
		}
	}
</script>

<Drawer bind:this={scriptEditorDrawer} size="1200px">
	<DrawerContent
		title="Script Editor"
		noPadding
		forceOverflowVisible
		on:close={scriptEditorDrawer.closeDrawer}
	>
		{#if selectedScript}
			<ScriptEditor
				lang={selectedScript.language}
				bind:code={selectedScript.content}
				path={selectedScript.path}
				bind:schema={selectedScript.schema}
				fixedOverflowWidgets={false}
			/>
		{/if}
		<svelte:fragment slot="actions">
			<Button startIcon={{ icon: faSave }} disabled>Automatically Saved</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>

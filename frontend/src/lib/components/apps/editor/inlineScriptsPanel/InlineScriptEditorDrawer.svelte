<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import type Editor from '$lib/components/Editor.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import type { InlineScript } from '../../types'
	import { Save } from 'lucide-svelte'

	let scriptEditorDrawer: Drawer
	export let inlineScript: InlineScript
	export let editor: Editor | undefined = undefined
	export let isOpen: boolean | undefined = undefined

	export function openDrawer() {
		scriptEditorDrawer.openDrawer?.()
	}
</script>

<Drawer bind:open={isOpen} bind:this={scriptEditorDrawer} size="1200px">
	<DrawerContent
		title="Script Editor"
		noPadding
		forceOverflowVisible
		on:close={() => {
			scriptEditorDrawer.closeDrawer()
			editor?.setCode(inlineScript.content)
		}}
	>
		{#if inlineScript && inlineScript.language != 'frontend'}
			<ScriptEditor
				noSyncFromGithub
				lang={inlineScript.language}
				path={inlineScript.path ? inlineScript.path + '_fullscreen' : undefined}
				fixedOverflowWidgets={false}
				bind:code={inlineScript.content}
				bind:schema={inlineScript.schema}
				tag={undefined}
			/>
		{/if}
		<svelte:fragment slot="actions">
			<Button startIcon={{ icon: Save }} disabled>Automatically Saved</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>

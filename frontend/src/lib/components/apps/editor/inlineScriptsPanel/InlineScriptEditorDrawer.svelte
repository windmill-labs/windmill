<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import type Editor from '$lib/components/Editor.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import type { InlineScript } from '../../types'
	import { Save } from 'lucide-svelte'

	let scriptEditorDrawer: Drawer | undefined = $state(undefined)
	interface Props {
		appPath: string
		inlineScript: InlineScript
		editor?: Editor | undefined
		isOpen?: boolean | undefined
		id: string
	}

	let {
		appPath,
		inlineScript = $bindable(),
		editor = undefined,
		isOpen = $bindable(undefined),
		id
	}: Props = $props()

	export function openDrawer() {
		scriptEditorDrawer?.openDrawer?.()
	}

	let args = $state.raw({})
</script>

<Drawer bind:open={isOpen} bind:this={scriptEditorDrawer} size="1200px">
	<DrawerContent
		title="Script Editor"
		noPadding
		forceOverflowVisible
		on:close={() => {
			scriptEditorDrawer?.closeDrawer()
			editor?.setCode(inlineScript.content)
		}}
	>
		{#if inlineScript && inlineScript.language != 'frontend'}
			<ScriptEditor
				showCaptures={false}
				noHistory
				noSyncFromGithub
				lang={inlineScript.language}
				path={appPath + '/' + id + '_fullscreen'}
				fixedOverflowWidgets={false}
				bind:code={inlineScript.content}
				bind:schema={inlineScript.schema}
				on:createScriptFromInlineScript
				tag={undefined}
				saveToWorkspace
				{args}
			/>
		{/if}
		{#snippet actions()}
			<Button size="xs" startIcon={{ icon: Save }} disabled>Automatically synced</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

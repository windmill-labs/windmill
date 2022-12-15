<script lang="ts">
	import type { Schema } from '$lib/common'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import { Preview } from '$lib/gen'
	import { DENO_INIT_CODE_CLEAR } from '$lib/script_helpers'
	import { emptySchema } from '$lib/utils'
	import { faSave } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'

	type InlineScript = {
		content: string
		language: Preview.language
		path: string
		schema: Schema
	}

	const { app, appPath } = getContext<AppEditorContext>('AppEditorContext')
	let scriptEditorDrawer: Drawer
	let selectedScript: InlineScript | undefined

	export function openDrawer(path: string) {
		if ($app.inlineScripts[path]) {
			selectedScript = $app.inlineScripts[path]
			scriptEditorDrawer.openDrawer?.()
		}
	}

	export function createScript(): string {
		let index = 0
		let newScriptPath = `inline_script_${index}`

		while ($app.inlineScripts?.[newScriptPath]) {
			newScriptPath = `inline_script_${++index}`
		}

		const path = `${appPath}/inline-script/inline_script_${index}`

		const inlineScript = {
			content: DENO_INIT_CODE_CLEAR,
			language: Preview.language.DENO,
			path,
			schema: emptySchema()
		}

		if ($app.inlineScripts) {
			$app.inlineScripts[newScriptPath] = inlineScript
		} else {
			$app.inlineScripts = {
				[newScriptPath]: inlineScript
			}
		}

		return newScriptPath
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

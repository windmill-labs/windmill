<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { Drawer } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Preview } from '$lib/gen'
	import { DENO_INIT_CODE_CLEAR } from '$lib/script_helpers'
	import { classNames, emptySchema } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'

	export let appPath: string

	const { connectingInput, staticOutputs, app, worldStore, selectedComponent } =
		getContext<AppEditorContext>('AppEditorContext')
	let newScriptPath: string
	let ignorePathError = false

	function connectInput(componentId: string, path: string) {
		if ($connectingInput) {
			$connectingInput = {
				opened: false,
				input: {
					connection: {
						componentId,
						path
					},
					type: 'connected'
				}
			}
		}
	}

	function createScript() {
		// To prevent the error message flashing up just before the drawer is closed
		ignorePathError = true
		const path = `${appPath}/inline-script/${newScriptPath}`
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
		scriptCreationDrawer?.closeDrawer?.()
		selectedScript = inlineScript
		scriptEditorDrawer.openDrawer?.()
	}

	let selectedScript:
		| { content: string; language: Preview.language; path: string; schema: Schema }
		| undefined = undefined
	let scriptEditorDrawer: Drawer

	let scriptCreationDrawer: Drawer | undefined = undefined
</script>

<PanelSection title="Outputs">
	{#each Object.entries($staticOutputs) as [componentId, outputs], index}
		{#if outputs.length > 0 && $worldStore?.outputsById[componentId]}
			<Button size="xs" on:click={() => ($selectedComponent = componentId)} color="blue">
				Component: {componentId}
			</Button>

			<div
				class={classNames(
					'w-full p-2 rounded-xs border',
					$selectedComponent === componentId
						? 'outline-1 outline outline-offset-2 outline-blue-500'
						: ''
				)}
			>
				<ComponentOutputViewer
					{outputs}
					{componentId}
					on:select={({ detail }) => {
						connectInput(componentId, detail)
					}}
				/>
			</div>
		{/if}
	{/each}
</PanelSection>

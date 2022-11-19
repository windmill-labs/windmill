<script lang="ts">
	import type { Schema } from '$lib/common'
	import { Drawer } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import { Preview } from '$lib/gen'
	import { DENO_INIT_CODE_CLEAR } from '$lib/script_helpers'
	import { emptySchema } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'

	export let appPath: string

	const { connectingInput, staticOutputs, app } = getContext<AppEditorContext>('AppEditorContext')

	function connectInput(id: string, name: string) {
		if ($connectingInput) {
			$connectingInput = {
				opened: false,
				input: {
					id,
					name,
					type: 'output',
					defaultValue: undefined
				}
			}
		}
	}

	function createScript() {
		const scriptPath = 'name'
		const path = `${appPath}/inline-script/${scriptPath}`
		const inlineScript = {
			content: DENO_INIT_CODE_CLEAR,
			language: Preview.language.DENO,
			path,
			schema: emptySchema()
		}

		if ($app.inlineScripts) {
			$app.inlineScripts[scriptPath] = inlineScript
		} else {
			$app.inlineScripts = {
				[scriptPath]: inlineScript
			}
		}
	}

	// Inline DENO, Inline Python, Inline GO, Inline SQL

	let selectedScript:
		| { content: string; language: Preview.language; path: string; schema: Schema }
		| undefined = undefined
	let scriptEditorDrawer: Drawer

	let scriptCreationDrawer: Drawer
</script>

<Drawer bind:this={scriptCreationDrawer} size="1000px">
	<DrawerContent
		title="Script creation"
		on:close={() => {
			scriptCreationDrawer.closeDrawer()
		}}
	>
		<input value="" />
		<Button on:click={createScript}>Create</Button>
	</DrawerContent>
</Drawer>

<Drawer bind:this={scriptEditorDrawer} size="1000px">
	<DrawerContent
		title="Script Editor"
		noPadding
		on:close={() => {
			scriptEditorDrawer.closeDrawer()
		}}
	>
		{#if selectedScript}
			<ScriptEditor
				lang={selectedScript.language}
				bind:code={selectedScript.content}
				path={selectedScript.path}
				bind:schema={selectedScript.schema}
			/>
		{/if}
	</DrawerContent>
</Drawer>

<PanelSection title="Component output">
	{#each Object.entries($staticOutputs) as [componentId, outputs], index}
		{#if outputs.length > 0}
			<Badge color="blue">{componentId}</Badge>

			{#each outputs as output}
				<Button
					size="xs"
					color="dark"
					disabled={!$connectingInput.opened}
					on:click={() => {
						connectInput(componentId, output)
					}}
				>
					{output}
				</Button>
			{/each}
		{/if}
	{/each}
</PanelSection>

<PanelSection title="Context">Todo</PanelSection>
<PanelSection title="Inline scripts">
	<Button
		size="xs"
		color="dark"
		on:click={() => {
			scriptCreationDrawer?.openDrawer()
		}}
		startIcon={{ icon: faPlus }}
	>
		<span>Add script</span>
	</Button>

	{#each $app.inlineScripts ? Object.entries($app.inlineScripts) : [] as [key, value]}
		<Button
			color="light"
			on:click={() => {
				if (value) {
					selectedScript = value
					scriptEditorDrawer.openDrawer()
				}
			}}
			size="xs"
		>
			{key}
		</Button>
	{/each}
</PanelSection>

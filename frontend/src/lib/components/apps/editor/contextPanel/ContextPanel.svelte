<script lang="ts">
	import { fade } from 'svelte/transition'
	import type { Schema } from '$lib/common'
	import { Drawer } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import { Preview } from '$lib/gen'
	import { DENO_INIT_CODE_CLEAR } from '$lib/script_helpers'
	import { classNames, emptySchema } from '$lib/utils'
	import { faEdit, faPlus, faSave, faTrash } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'

	export let appPath: string

	const { connectingInput, staticOutputs, app, worldStore, selectedComponent } =
		getContext<AppEditorContext>('AppEditorContext')
	let newScriptPath: string
	let ignorePathError = false

	$: isTakenPath = Object.keys($app.inlineScripts).includes(newScriptPath)

	function connectInput(componentId: string, path: string) {
		if ($connectingInput) {
			debugger
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

	function afterCreateScript() {
		newScriptPath = ''
		ignorePathError = false
	}

	let selectedScript:
		| { content: string; language: Preview.language; path: string; schema: Schema }
		| undefined = undefined
	let scriptEditorDrawer: Drawer

	let scriptCreationDrawer: Drawer | undefined = undefined
</script>

<Drawer bind:this={scriptCreationDrawer} size="600px" on:afterClose={afterCreateScript}>
	<DrawerContent title="Script creation" on:close={scriptCreationDrawer.closeDrawer}>
		<label for="pathInput" class="text-sm font-semibold"> Script name </label>
		<div class="flex justify-between items-center gap-4">
			<!-- svelte-ignore a11y-autofocus -->
			<input
				autofocus
				id="pathInput"
				class="grow min-w-[150px]"
				bind:value={newScriptPath}
				on:keypress={(e) => e.key === 'Enter' && createScript()}
			/>
			<Button on:click={createScript} size="sm" disabled={isTakenPath} startIcon={{ icon: faPlus }}>
				Create
			</Button>
		</div>
		{#if isTakenPath && !ignorePathError}
			<div transition:fade={{ duration: 100 }} class="text-sm text-red-600 h-5 mt-1">
				This name is already used.
			</div>
		{/if}
	</DrawerContent>
</Drawer>

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

<PanelSection title="Inline scripts">
	<svelte:fragment slot="action">
		<Button
			size="xs"
			color="light"
			variant="border"
			on:click={scriptCreationDrawer?.openDrawer}
			startIcon={{ icon: faPlus }}
			iconOnly
		/>
	</svelte:fragment>

	<div class="w-full border rounded-sm">
		{#each $app.inlineScripts ? Object.entries($app.inlineScripts) : [] as [key, value], index}
			<div
				class={classNames(
					'flex justify-between flex-row w-full items-center p-2',
					index % 2 === 0 ? 'bg-gray-100' : 'bg-white'
				)}
			>
				<span class="text-xs">{key}</span>
				<div class="flex gap-2">
					<Button
						size="xs"
						color="light"
						variant="border"
						iconOnly
						startIcon={{ icon: faEdit }}
						on:click={() => {
							if (value) {
								selectedScript = value
								scriptEditorDrawer.openDrawer && scriptEditorDrawer.openDrawer()
							}
						}}
					/>
					<Button
						size="xs"
						color="red"
						variant="border"
						iconOnly
						startIcon={{ icon: faTrash }}
						on:click={() => {
							if ($app.inlineScripts[key]) {
								delete $app.inlineScripts[key]
								$app = $app
							}
						}}
					/>
				</div>
			</div>
		{/each}
	</div>
</PanelSection>

<PanelSection title="Outputs">
	{#each Object.entries($staticOutputs) as [componentId, outputs], index}
		{#if outputs.length > 0 && $worldStore?.outputsById[componentId]}
			<Button
				btnClasses="bg-blue-100 text-blue-800 hover:bg-blue-200 !px-2 !py-1 text-xs focus:bg-blue-200"
				size="xs"
				spacingSize="xs"
				on:click={() => ($selectedComponent = componentId)}
				color="blue"
			>
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

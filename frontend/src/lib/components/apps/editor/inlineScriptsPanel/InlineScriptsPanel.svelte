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
	import { faPlus, faSave, faTrash } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane } from 'svelte-splitpanes'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import { Code2 } from 'lucide-svelte'

	export let appPath: string

	const { app } = getContext<AppEditorContext>('AppEditorContext')
	let newScriptPath: string
	let ignorePathError = false

	$: isTakenPath = Object.keys($app.inlineScripts).includes(newScriptPath)

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

	$: selectedScript = undefined as
		| { content: string; language: Preview.language; path: string; schema: Schema }
		| undefined
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

<SplitPanesWrapper>
	<Pane size={25}>
		<PanelSection title="Inline scripts">
			<div class="flex flex-col gap-2 w-full">
				{#if $app.inlineScripts && Object.keys($app.inlineScripts).length > 0}
					<div class="flex gap-2 flex-col ">
						{#each $app.inlineScripts ? Object.entries($app.inlineScripts) : [] as [key, value], index}
							<!-- svelte-ignore a11y-click-events-have-key-events -->
							<div
								class="{classNames(
									'border flex justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
									selectedScript?.path === value.path ? 'bg-blue-100 text-blue-600' : ''
								)},"
								on:click={() => {
									selectedScript = value
								}}
							>
								<span class="text-xs">{key}</span>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-sm text-gray-500">No inline scripts</div>
				{/if}
			</div>
		</PanelSection>
	</Pane>
	<Pane size={75}>
		{#key selectedScript?.path}
			{#if selectedScript}
				<div class="p-4 flex flex-col gap-2 ">
					<div class="flex w-full flex-row-reverse gap-2 items-center">
						<Button
							size="xs"
							color="light"
							variant="border"
							on:click={() => {
								scriptEditorDrawer.openDrawer && scriptEditorDrawer.openDrawer()
							}}
						>
							<div class="flex gap-1 items-center">
								<Code2 size={16} />
								Open full editor
							</div>
						</Button>
						<Button
							size="xs"
							color="light"
							variant="border"
							iconOnly
							startIcon={{ icon: faTrash }}
							on:click={() => {
								const key = Object.keys($app.inlineScripts).find(
									(key) => $app.inlineScripts[key].path === selectedScript?.path
								)

								if (key && $app.inlineScripts[key]) {
									delete $app.inlineScripts[key]
									$app = $app
									selectedScript = undefined
								}
							}}
						/>
						<div class="text-xs italic">{selectedScript?.path}</div>
					</div>

					<div class="border">
						<SimpleEditor
							class="few-lines-editor"
							lang="typescript"
							bind:code={selectedScript.content}
							fixedOverflowWidgets={false}
						/>
					</div>
				</div>
			{/if}
		{/key}
	</Pane>
</SplitPanesWrapper>

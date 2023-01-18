<script lang="ts">
	import { faMousePointer, faPlus } from '@fortawesome/free-solid-svg-icons'
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import { Building, Globe2 } from 'lucide-svelte'
	import InlineScriptList from './InlineScriptList.svelte'
	import type { ResultAppInput } from '$lib/components/apps/inputType'
	import WorkspaceScriptList from './WorkspaceScriptList.svelte'
	import WorkspaceFlowList from './WorkspaceFlowList.svelte'
	import type { AppEditorContext, GridItem, InlineScript } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import type { Schema } from '$lib/common'
	import { getAllScriptNames, loadSchema, schemaToInputsSpec } from '$lib/components/apps/utils'
	import { emptySchema } from '$lib/utils'

	type Tab = 'hubscripts' | 'workspacescripts' | 'workspaceflows' | 'inlinescripts'

	export let appInput: ResultAppInput
	export let defaultUserInput = false

	let tab: Tab = 'inlinescripts'
	let filter: string = ''
	let picker: Drawer

	const { app, workspace } = getContext<AppEditorContext>('AppEditorContext')

	async function loadSchemaFromTriggerable(
		path: string,
		runType: 'script' | 'flow' | 'hubscript'
	): Promise<Schema> {
		return loadSchema(workspace, path, runType) ?? emptySchema()
	}

	async function pickScript(path: string) {
		if (appInput.type === 'runnable') {
			const schema = await loadSchemaFromTriggerable(path, 'script')
			const fields = schemaToInputsSpec(schema, defaultUserInput)
			appInput.runnable = {
				type: 'runnableByPath',
				path,
				runType: 'script',
				schema
			}
			appInput.fields = fields
		}
	}

	async function pickFlow(path: string) {
		if (appInput.type === 'runnable') {
			const schema = await loadSchemaFromTriggerable(path, 'flow')
			const fields = schemaToInputsSpec(schema, defaultUserInput)
			appInput.runnable = {
				type: 'runnableByPath',
				path,
				runType: 'flow',
				schema
			}
			appInput.fields = fields
		}
	}

	async function pickHubScript(path: string) {
		if (appInput.type === 'runnable') {
			const schema = await loadSchemaFromTriggerable(path, 'hubscript')
			const fields = schemaToInputsSpec(schema, defaultUserInput)
			appInput.runnable = {
				type: 'runnableByPath',
				path,
				runType: 'hubscript',
				schema
			}
			appInput.fields = fields
		}
	}

	function pickInlineScript(name: string) {
		const unusedInlineScriptIndex = $app.unusedInlineScripts?.findIndex(
			(script) => script.name === name
		)
		const unusedInlineScript = $app.unusedInlineScripts?.[unusedInlineScriptIndex]
		if (appInput.type === 'runnable' && unusedInlineScript?.inlineScript) {
			appInput.runnable = {
				type: 'runnableByName',
				name,
				inlineScript: unusedInlineScript.inlineScript
			}

			$app.unusedInlineScripts.splice(unusedInlineScriptIndex, 1)
		}
		$app = $app
	}

	function createScript(): string {
		let index = 0
		let newScriptPath = `Inline Script ${index}`

		const names = getAllScriptNames($app)

		// Find a name that is not used by any other inline script
		while (names.includes(newScriptPath)) {
			newScriptPath = `Inline Script ${++index}`
		}

		appInput.runnable = {
			type: 'runnableByName',
			name: newScriptPath,
			inlineScript: undefined
		}

		appInput = appInput

		return newScriptPath
	}
</script>

<Drawer bind:this={picker} size="1000px">
	<DrawerContent title="Script/Flow Picker" on:close={picker.closeDrawer}>
		<div>
			<div class="max-w-6xl">
				<Tabs bind:selected={tab}>
					<Tab size="sm" value="inlinescripts">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Detached Inline Scripts
						</div>
					</Tab>
					<Tab size="sm" value="workspacescripts">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Workspace Scripts
						</div>
					</Tab>
					<Tab size="sm" value="workspaceflows">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Workspace Flows
						</div>
					</Tab>
					<Tab size="sm" value="hubscripts">
						<div class="flex gap-2 items-center my-1">
							<Globe2 size={18} />
							Hub Scripts
						</div>
					</Tab>
				</Tabs>
				<div class="my-2" />
				<div class="flex flex-col gap-y-16">
					<div class="flex flex-col">
						{#if tab == 'inlinescripts'}
							<InlineScriptList
								on:pick={(e) => pickInlineScript(e.detail)}
								inlineScripts={$app.unusedInlineScripts
									? $app.unusedInlineScripts.map((uis) => uis.name)
									: []}
							/>
						{:else if tab == 'workspacescripts'}
							<WorkspaceScriptList on:pick={(e) => pickScript(e.detail)} />
						{:else if tab == 'workspaceflows'}
							<WorkspaceFlowList on:pick={(e) => pickFlow(e.detail)} />
						{:else if tab == 'hubscripts'}
							<PickHubScript bind:filter on:pick={(e) => pickHubScript(e.detail.path)} />
						{/if}
					</div>
				</div>
			</div>
		</div>
	</DrawerContent>
</Drawer>

<div class="flex flex-col gap-2">
	<Button
		on:click={createScript}
		size="sm"
		color="light"
		variant="border"
		startIcon={{ icon: faPlus }}
		btnClasses="truncate"
	>
		Create an inline script
	</Button>
	<Button
		on:click={() => picker?.openDrawer()}
		size="sm"
		color="blue"
		startIcon={{ icon: faMousePointer }}
		btnClasses="truncate"
	>
		Select a script or flow
	</Button>
</div>

<script lang="ts">
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import { Building, Globe2, MousePointer, Plus } from 'lucide-svelte'
	import InlineScriptList from './InlineScriptList.svelte'
	import type { Runnable, StaticAppInput } from '$lib/components/apps/inputType'
	import WorkspaceScriptList from './WorkspaceScriptList.svelte'
	import WorkspaceFlowList from './WorkspaceFlowList.svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { Schema } from '$lib/common'
	import { getAllScriptNames, schemaToInputsSpec } from '$lib/components/apps/utils'
	import { defaultIfEmptyString, emptySchema } from '$lib/utils'
	import { loadSchema } from '$lib/infer'

	type Tab = 'hubscripts' | 'workspacescripts' | 'workspaceflows' | 'inlinescripts'

	export let defaultUserInput = false
	export let hideCreateScript = false
	export let onlyFlow = false

	let tab: Tab = onlyFlow ? 'workspaceflows' : 'inlinescripts'
	let filter: string = ''
	let picker: Drawer

	const { app, workspace } = getContext<AppViewerContext>('AppViewerContext')

	const dispatch = createEventDispatcher<{
		pick: {
			runnable: Runnable
			fields: Record<string, StaticAppInput>
		}
	}>()

	async function loadSchemaFromTriggerable(
		path: string,
		runType: 'script' | 'flow' | 'hubscript'
	): Promise<{ schema: Schema; summary: string | undefined }> {
		return loadSchema(workspace, path, runType) ?? emptySchema()
	}

	async function pickScript(path: string) {
		const schema = await loadSchemaFromTriggerable(path, 'script')
		const fields = schemaToInputsSpec(schema.schema, defaultUserInput)
		const runnable = {
			type: 'runnableByPath',
			path,
			runType: 'script',
			schema,
			name: defaultIfEmptyString(schema.summary, path)
		} as const

		dispatch('pick', {
			runnable,
			fields
		})
	}

	async function pickFlow(path: string) {
		const schema = await loadSchemaFromTriggerable(path, 'flow')
		const fields = schemaToInputsSpec(schema.schema, defaultUserInput)
		const runnable = {
			type: 'runnableByPath',
			path,
			runType: 'flow',
			schema,
			name: defaultIfEmptyString(schema.summary, path)
		} as const
		dispatch('pick', {
			runnable,
			fields
		})
	}

	async function pickHubScript(path: string) {
		const schema = await loadSchemaFromTriggerable(path, 'hubscript')
		const fields = schemaToInputsSpec(schema.schema, defaultUserInput)
		const runnable = {
			type: 'runnableByPath',
			path,
			runType: 'hubscript',
			schema,
			name: defaultIfEmptyString(schema.summary, path)
		} as const
		dispatch('pick', {
			runnable,
			fields
		})
	}

	function pickInlineScript(name: string) {
		const unusedInlineScriptIndex = $app.unusedInlineScripts?.findIndex(
			(script) => script.name === name
		)
		const unusedInlineScript = $app.unusedInlineScripts?.[unusedInlineScriptIndex]
		dispatch('pick', {
			runnable: {
				type: 'runnableByName',
				name,
				inlineScript: unusedInlineScript.inlineScript
			},
			fields: {}
		})

		$app.unusedInlineScripts.splice(unusedInlineScriptIndex, 1)
		$app.unusedInlineScripts = $app.unusedInlineScripts
	}

	function createScript() {
		let index = 0
		let newScriptPath = `Inline Script ${index}`

		const names = getAllScriptNames($app)

		// Find a name that is not used by any other inline script
		while (names.includes(newScriptPath)) {
			newScriptPath = `Inline Script ${++index}`
		}

		dispatch('pick', {
			runnable: {
				type: 'runnableByName',
				name: newScriptPath,
				inlineScript: undefined
			},
			fields: {}
		})
	}
</script>

<Drawer bind:this={picker} size="1000px">
	<DrawerContent title="Script/Flow Picker" on:close={picker.closeDrawer}>
		<div>
			<div class="max-w-6xl">
				<Tabs bind:selected={tab}>
					{#if !onlyFlow}
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
					{/if}
					<Tab size="sm" value="workspaceflows">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Workspace Flows
						</div>
					</Tab>
					{#if !onlyFlow}
						<Tab size="sm" value="hubscripts">
							<div class="flex gap-2 items-center my-1">
								<Globe2 size={18} />
								Hub Scripts
							</div>
						</Tab>
					{/if}
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
	{#if !hideCreateScript}
		<Button
			on:click={createScript}
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: Plus }}
			btnClasses="truncate w-full"
			id="app-editor-create-inline-script"
		>
			Create an inline script
		</Button>
	{/if}
	<Button
		on:click={() => picker?.openDrawer()}
		size="xs"
		color="blue"
		variant="border"
		startIcon={{ icon: MousePointer }}
		btnClasses="truncate w-full"
	>
		{#if onlyFlow}Select a flow{:else}Select a script or flow{/if}
	</Button>
</div>

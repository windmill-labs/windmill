<script lang="ts">
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import { Building, Globe2, MousePointer, Plus } from 'lucide-svelte'
	import InlineScriptList from './InlineScriptList.svelte'
	import type { InlineScript, Runnable, StaticAppInput } from '$lib/components/apps/inputType'
	import WorkspaceScriptList from './WorkspaceScriptList.svelte'
	import WorkspaceFlowList from './WorkspaceFlowList.svelte'
	import { createEventDispatcher } from 'svelte'
	import type { Schema } from '$lib/common'
	import { schemaToInputsSpec } from '$lib/components/apps/utils'
	import { defaultIfEmptyString, emptySchema } from '$lib/utils'
	import { loadSchema } from '$lib/infer'
	import { workspaceStore } from '$lib/stores'

	type TabType = 'hubscripts' | 'workspacescripts' | 'workspaceflows' | 'inlinescripts'

	interface Props {
		defaultUserInput?: boolean
		hideCreateScript?: boolean
		onlyFlow?: boolean
		rawApps?: boolean
		unusedInlineScripts: { name: string; inlineScript: InlineScript }[]
	}

	let {
		defaultUserInput = false,
		hideCreateScript = false,
		onlyFlow = false,
		rawApps = false,
		unusedInlineScripts = $bindable()
	}: Props = $props()

	// const { app, workspace } = getContext<AppViewerContext>('AppViewerContext')

	let tab: TabType = $state(
		onlyFlow
			? 'workspaceflows'
			: unusedInlineScripts?.length > 0
				? 'inlinescripts'
				: 'workspacescripts'
	)
	let filter: string = $state('')
	let picker: Drawer | undefined = $state()

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
		const schema = await loadSchema($workspaceStore!, path, runType)
		if (!schema.schema.order) {
			schema.schema.order = Object.keys(schema.schema.properties ?? {})
		}
		return schema ?? { schema: emptySchema(), summary: undefined }
	}

	async function pickScript(path: string) {
		const schema = await loadSchemaFromTriggerable(path, 'script')
		const fields = schemaToInputsSpec(schema.schema, defaultUserInput)
		const runnable = {
			type: 'path',
			path,
			runType: 'script',
			schema: schema.schema,
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
			type: 'path',
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
			type: 'path',
			path,
			runType: 'hubscript',
			schema: schema.schema,
			name: defaultIfEmptyString(schema.summary, path)
		} as const
		dispatch('pick', {
			runnable,
			fields
		})
	}

	function pickInlineScript(name: string) {
		const unusedInlineScriptIndex = unusedInlineScripts?.findIndex((script) => script.name === name)
		const unusedInlineScript = unusedInlineScripts?.[unusedInlineScriptIndex]
		dispatch('pick', {
			runnable: {
				type: 'inline',
				name,
				inlineScript: unusedInlineScript.inlineScript
			},
			fields: {}
		})

		unusedInlineScripts.splice(unusedInlineScriptIndex, 1)
		unusedInlineScripts = unusedInlineScripts
	}

	function createScript() {
		let newScriptName = `Inline Script`

		dispatch('pick', {
			runnable: {
				type: 'inline',
				name: newScriptName,
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
						{#if !rawApps}
							<Tab value="inlinescripts" label="Detached Inline Scripts" icon={Building} />
						{/if}
						<Tab value="workspacescripts" label="Workspace Scripts" icon={Building} />
					{/if}
					<Tab value="workspaceflows" label="Workspace Flows" icon={Building} />
					{#if !onlyFlow}
						<Tab value="hubscripts" label="Hub Scripts" icon={Globe2} />
					{/if}
				</Tabs>
				<div class="my-2"></div>
				<div class="flex flex-col gap-y-16">
					<div class="flex flex-col">
						{#if tab == 'inlinescripts'}
							<InlineScriptList
								on:pick={(e) => pickInlineScript(e.detail)}
								inlineScripts={unusedInlineScripts
									? unusedInlineScripts.map((uis) => uis.name)
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
			unifiedSize="md"
			variant="default"
			startIcon={{ icon: Plus }}
			btnClasses="truncate w-full"
			id="app-editor-create-inline-script"
		>
			Create an inline script
		</Button>
	{/if}
	<Button
		on:click={() => picker?.openDrawer()}
		unifiedSize="md"
		variant={rawApps ? 'contained' : 'border'}
		startIcon={{ icon: MousePointer }}
		btnClasses="truncate w-full"
	>
		{#if onlyFlow}Select a flow{:else}Select a script or flow{/if}
	</Button>
</div>

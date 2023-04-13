<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import FlowModuleScript from '$lib/components/flows/content/FlowModuleScript.svelte'
	import FlowPathViewer from '$lib/components/flows/content/FlowPathViewer.svelte'
	import { inferArgs } from '$lib/infer'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, getScriptByPath, sendUserToast } from '$lib/utils'
	import {
		faCodeBranch,
		faExternalLinkAlt,
		faEye,
		faPen,
		faRefresh
	} from '@fortawesome/free-solid-svg-icons'
	import type { AppInput, RunnableByPath } from '../../inputType'
	import { clearResultAppInput, loadSchema } from '../../utils'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import { computeFields } from './utils'
	import { deepEqual } from 'fast-equals'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'

	export let componentInput: AppInput | undefined
	export let defaultUserInput = false
	export let componentType: string
	export let id: string
	export let transformer: boolean

	const { app, stateId } = getContext<AppViewerContext>('AppViewerContext')

	async function fork(path: string) {
		let { content, language, schema } = await getScriptByPath(path)
		if (componentInput && componentInput.type == 'runnable') {
			if (!schema || typeof schema == 'string' || Object.keys(schema).length == 0) {
				schema = emptySchema()
				await inferArgs(language, content, schema)
			}
			componentInput.runnable = {
				type: 'runnableByName',
				name: path,
				inlineScript: {
					content,
					language,
					schema,
					path
				}
			}
		} else {
			console.error('componentInput is undefined')
		}
	}

	let drawerFlowViewer: Drawer
	let flowPath: string = ''

	async function refreshScript(x: RunnableByPath) {
		let { schema } = await getScriptByPath(x.path)
		if (!deepEqual(x.schema, schema)) {
			x.schema = schema
			if (componentInput?.type == 'runnable') {
				componentInput.fields = computeFields(schema, false, componentInput.fields)
			}
			componentInput = componentInput
		}
	}

	async function refreshFlow(x: RunnableByPath) {
		const schema = (await loadSchema($workspaceStore ?? '', x.path, 'flow')) ?? emptySchema()
		x.schema = schema
		if (componentInput?.type == 'runnable') {
			componentInput.fields = computeFields(schema, false, componentInput.fields)
		}
		componentInput = componentInput
	}
	$: if (
		componentInput &&
		componentInput.type == 'runnable' &&
		componentInput?.runnable?.type === 'runnableByPath'
	) {
		if (componentInput.runnable.runType == 'script') {
			refreshScript(componentInput.runnable)
		} else if (componentInput.runnable.runType == 'flow') {
			refreshFlow(componentInput.runnable)
		}
	}

	function deleteInlineScript() {
		if (componentInput && componentInput.type == 'runnable') {
			componentInput = clearResultAppInput(componentInput)
		}
	}
</script>

<Drawer bind:this={drawerFlowViewer} size="1200px">
	<DrawerContent title="Flow {flowPath}" on:close={drawerFlowViewer.closeDrawer}>
		<FlowPathViewer path={flowPath ?? ''} />
	</DrawerContent>
</Drawer>

{#if transformer}
	{#if componentInput?.type == 'runnable' && componentInput.transformer}
		<InlineScriptEditor
			transformer
			defaultUserInput={false}
			{id}
			bind:inlineScript={componentInput.transformer}
			name="Transformer"
			on:delete={() => {
				if (componentInput?.type == 'runnable') {
					componentInput.transformer = undefined
					componentInput = componentInput
				}
			}}
		/>
	{:else}
		<span class="px-2 text-gray-600">
			Selected editor component is a transformer but component has no transformer
		</span>
	{/if}
{:else if componentInput && componentInput.type == 'runnable'}
	{#if componentInput?.runnable?.type === 'runnableByName' && componentInput?.runnable?.name !== undefined}
		{#if componentInput.runnable.inlineScript}
			<InlineScriptEditor
				{defaultUserInput}
				{id}
				bind:inlineScript={componentInput.runnable.inlineScript}
				bind:name={componentInput.runnable.name}
				bind:fields={componentInput.fields}
				syncFields
				on:delete={deleteInlineScript}
			/>
		{:else}
			<EmptyInlineScript
				{componentType}
				name={componentInput.runnable.name}
				on:delete={deleteInlineScript}
				on:new={(e) => {
					if (
						componentInput &&
						componentInput.type == 'runnable' &&
						componentInput?.runnable?.type === 'runnableByName'
					) {
						componentInput.runnable.inlineScript = e.detail
						$app = $app
					}
				}}
			/>
		{/if}
	{:else if componentInput?.runnable?.type === 'runnableByPath' && componentInput?.runnable?.path}
		<div class="p-2 h-full flex flex-col gap-2">
			{#if componentInput.runnable.runType == 'script' || componentInput.runnable.runType == 'hubscript'}
				<div>
					<Button
						size="xs"
						startIcon={{ icon: faCodeBranch }}
						on:click={() => {
							if (
								componentInput &&
								componentInput.type == 'runnable' &&
								componentInput.runnable?.type === 'runnableByPath'
							) {
								fork(componentInput.runnable.path)
								$app = $app
							}
						}}
					>
						Fork
					</Button>
				</div>
			{/if}
			<div class="w-full">
				{#key $stateId}
					{#if componentInput.runnable.runType == 'script' || componentInput.runnable.runType == 'hubscript'}
						<FlowModuleScript path={componentInput.runnable.path} />
					{:else if componentInput.runnable.runType == 'flow'}
						<div class="pb-2 flex gap-2 w-full flex-row-reverse">
							<Button
								size="xs"
								startIcon={{ icon: faRefresh }}
								on:click={() => {
									if (
										componentInput?.type == 'runnable' &&
										componentInput.runnable?.type === 'runnableByPath'
									) {
										sendUserToast('Refreshing inputs')
										if (componentInput.runnable.runType == 'script') {
											refreshScript(componentInput.runnable)
										} else if (componentInput.runnable.runType == 'flow') {
											refreshFlow(componentInput.runnable)
										}
										$stateId = $stateId + 1
									}
								}}
							>
								Refresh
							</Button>
							<Button
								size="xs"
								startIcon={{ icon: faEye }}
								on:click={() => {
									flowPath = componentInput?.['runnable']?.path
									drawerFlowViewer.openDrawer()
								}}
							>
								Expand
							</Button>
							<Button
								size="xs"
								startIcon={{ icon: faPen }}
								endIcon={{ icon: faExternalLinkAlt }}
								target="_blank"
								href="/flows/edit/{componentInput?.['runnable']?.path}?nodraft=true">Edit</Button
							>
							<Button
								size="xs"
								startIcon={{ icon: faEye }}
								endIcon={{ icon: faExternalLinkAlt }}
								target="_blank"
								href="/flows/get/{componentInput?.['runnable']?.path}?workspace={$workspaceStore}"
							>
								Details page
							</Button>
						</div>
						<FlowPathViewer path={componentInput.runnable.path} />
					{:else}
						Unrecognized runType {componentInput.runnable.runType}
					{/if}
				{/key}
			</div>
		</div>
	{/if}
{/if}

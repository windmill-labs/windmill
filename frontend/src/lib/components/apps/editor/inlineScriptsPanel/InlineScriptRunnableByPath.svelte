<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import { base } from '$lib/base'
	import FlowGraphViewer from '$lib/components/FlowGraphViewer.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import FlowModuleScript from '$lib/components/flows/content/FlowModuleScript.svelte'
	import FlowPathViewer from '$lib/components/flows/content/FlowPathViewer.svelte'
	import { emptySchema, getHubFlowIdFromPath, isHubFlowPath, sendUserToast } from '$lib/utils'
	import { getContext, tick, untrack } from 'svelte'
	import type {
		ConnectedAppInput,
		RowAppInput,
		RunnableByPath,
		StaticAppInput,
		UserAppInput,
		CtxAppInput
	} from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import { computeFields } from './utils'
	import { inferArgs, loadSchema } from '$lib/infer'
	import AppRunButton from './AppRunButton.svelte'
	import { getScriptByPath } from '$lib/scripts'
	import { autoPlacement } from '@floating-ui/core'
	import { ExternalLink, Eye, GitFork, Pen, RefreshCw, Trash } from 'lucide-svelte'
	import { get } from 'svelte/store'
	import RunButton from '$lib/components/RunButton.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import ScriptEditorDrawer from '$lib/components/flows/content/ScriptEditorDrawer.svelte'
	import FlowEditorDrawer from '$lib/components/flows/content/FlowEditorDrawer.svelte'
	import { FlowService, ScriptService, type OpenFlow } from '$lib/gen'
	import { replaceScriptPlaceholderWithItsValues } from '$lib/hub'

	interface Props {
		runnable: RunnableByPath
		fields:
			| Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput | CtxAppInput>
			| undefined
		id: string
		rawApps?: boolean
		isLoading?: boolean
		onRun?: any
		onCancel?: any
		hubFlowPreview?: OpenFlow | undefined
	}

	let {
		runnable = $bindable(),
		fields = $bindable(),
		id,
		rawApps = false,
		isLoading = false,
		onRun = async () => {},
		onCancel = async () => {},
		hubFlowPreview = $bindable(undefined)
	}: Props = $props()

	const viewerContext = getContext<AppViewerContext>('AppViewerContext')

	let drawerFlowViewer: Drawer | undefined = $state(undefined)
	let flowPath: string = $state('')
	let drawerHubFlowPreview: (OpenFlow & { path?: string }) | undefined = $state(undefined)
	let notFound = $state(false)

	// Key to force re-mounting of viewer components (bypasses FlowModuleScript cache)
	let refreshKey = $state(0)

	let scriptEditorDrawer: ScriptEditorDrawer | undefined = $state(undefined)
	let flowEditorDrawer: FlowEditorDrawer | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	async function refreshScript(runnable: RunnableByPath) {
		hubFlowPreview = undefined
		try {
			let { schema } = await getScriptByPath(runnable.path)
			if (!deepEqual(runnable.schema, schema)) {
				runnable.schema = schema
				if (!runnable.schema.order) {
					runnable.schema.order = Object.keys(runnable.schema.properties ?? {})
				}
				fields = computeFields(schema, false, fields ?? {})
			}
		} catch (e) {
			notFound = true
			console.error(e)
		}
	}

	async function refreshFlow(runnable: RunnableByPath) {
		hubFlowPreview = undefined
		try {
			const hubFlowId = getHubFlowIdFromPath(runnable.path)
			if (hubFlowId !== undefined) {
				const hub = await FlowService.getHubFlowById({ id: hubFlowId })
				const flow = hub.flow ? structuredClone(hub.flow) : undefined
				if (flow?.value.preprocessor_module?.value.type === 'rawscript') {
					flow.value.preprocessor_module.value.content = replaceScriptPlaceholderWithItsValues(
						String(hubFlowId),
						flow.value.preprocessor_module.value.content
					)
				}

				if (!flow) {
					notFound = true
					return
				}

				hubFlowPreview = flow
				const schema =
					flow.schema && typeof flow.schema === 'object' && Object.keys(flow.schema).length > 0
						? (flow.schema as any)
						: emptySchema()
				if (!deepEqual(runnable.schema, schema)) {
					runnable.schema = schema
					if (!runnable.schema.order) {
						runnable.schema.order = Object.keys(runnable.schema.properties ?? {})
					}
					fields = computeFields(schema, false, fields ?? {})
				}
				return
			}

			const { schema } =
				(await loadSchema($workspaceStore ?? '', runnable.path, 'flow')) ?? emptySchema()
			if (!deepEqual(runnable.schema, schema)) {
				runnable.schema = schema
				if (!runnable.schema.order) {
					runnable.schema.order = Object.keys(runnable.schema.properties ?? {})
				}
				fields = computeFields(schema, false, fields ?? {})
			}
		} catch (e) {
			notFound = true
			console.error(e)
		}
	}

	async function fork(path: string) {
		let { content, language, schema } = await getScriptByPath(path)
		if (!schema || typeof schema == 'string' || Object.keys(schema).length == 0) {
			schema = emptySchema()
			await inferArgs(language, content, schema)
		}
		dispatch('fork', {
			type: 'inline',
			name: path,
			inlineScript: {
				content,
				language,
				schema,
				path
			}
		})
	}

	async function openScriptEditor(path: string) {
		try {
			const script = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path
			})
			scriptEditorDrawer?.openDrawer(script.hash, () => {
				// Increment refreshKey to force re-mounting of FlowModuleScript (bypasses cache)
				refreshKey++
				// Refresh the schema
				lastRunnable = undefined
				refresh(runnable)
			})
		} catch (e) {
			sendUserToast(`Failed to load script: ${e}`, true)
		}
	}

	function openFlowEditor(path: string) {
		flowEditorDrawer?.openDrawer(path, () => {
			// Increment refreshKey to force re-mounting of FlowPathViewer (bypasses cache)
			refreshKey++
			// Refresh the schema
			lastRunnable = undefined
			refresh(runnable)
		})
	}

	let lastRunnable: RunnableByPath | undefined = undefined
	function refresh(runnable) {
		if (deepEqual(runnable, lastRunnable)) {
			return
		}
		notFound = false
		if (runnable.runType == 'script') {
			refreshScript(runnable)
		} else if (runnable.runType == 'flow') {
			refreshFlow(runnable)
		} else {
			hubFlowPreview = undefined
		}
		lastRunnable = runnable
	}
	$effect(() => {
		runnable.path
		untrack(() => {
			refresh(runnable)
		})
	})
</script>

<Drawer bind:this={drawerFlowViewer} size="1200px">
	<DrawerContent
		title="Flow {flowPath}"
		on:close={() => {
			drawerHubFlowPreview = undefined
			drawerFlowViewer?.closeDrawer()
		}}
	>
		{#if drawerHubFlowPreview}
			<div class="flex flex-col flex-1 h-full overflow-auto">
				<FlowGraphViewer triggerNode provideTriggerContext flow={drawerHubFlowPreview} />
			</div>
		{:else}
			<FlowPathViewer path={flowPath ?? ''} />
		{/if}
	</DrawerContent>
</Drawer>

<ScriptEditorDrawer
	bind:this={scriptEditorDrawer}
	on:save={() => {
		// Increment refreshKey to force re-mounting of FlowModuleScript (bypasses cache)
		refreshKey++
		// Refresh the schema
		lastRunnable = undefined
		refresh(runnable)
	}}
/>

<FlowEditorDrawer
	bind:this={flowEditorDrawer}
	on:save={() => {
		// Increment refreshKey to force re-mounting of FlowPathViewer (bypasses cache)
		refreshKey++
		// Refresh the schema
		lastRunnable = undefined
		refresh(runnable)
	}}
/>

<div class="p-2 h-full flex flex-col gap-2">
	<div class="flex flex-row-reverse w-full gap-2">
		{#if !rawApps}
			<AppRunButton hideShortcut {id} />
		{:else}
			<RunButton {isLoading} {onRun} {onCancel} />
		{/if}

			<Button
				variant="default"
				size="xs"
				startIcon={{ icon: RefreshCw }}
				on:click={async () => {
					sendUserToast('Getting latest runnable version at that path')
					// Increment refreshKey to force re-mounting of viewer components (bypasses cache)
					refreshKey++
					lastRunnable = undefined
				refresh(runnable)
				if (viewerContext) {
					viewerContext.stateId.update((x) => x + 1)
				}
				await tick()
			}}
		/>
		<Button
			size="xs"
			variant="default"
			startIcon={{ icon: Trash }}
			on:click={() => {
				dispatch('delete')
			}}
		>
				Clear
			</Button>
			{#if runnable.runType == 'flow'}
				<Button
					variant="default"
					size="xs"
					startIcon={{ icon: Eye }}
					on:click={() => {
						flowPath = runnable.path
						drawerHubFlowPreview = isHubFlowPath(runnable.path) && hubFlowPreview
							? { ...hubFlowPreview, path: runnable.path }
							: undefined
						drawerFlowViewer?.openDrawer()
					}}
				>
					Expand
				</Button>
				{#if getHubFlowIdFromPath(runnable.path)}
					<Button
						variant="default"
						size="xs"
						startIcon={{ icon: GitFork }}
						endIcon={{ icon: ExternalLink }}
						target="_blank"
						href="{base}/flows/add?hub={getHubFlowIdFromPath(runnable.path)}"
					>
						Fork
					</Button>
				{:else}
					<Button
						variant="default"
						size="xs"
						startIcon={{ icon: Pen }}
						on:click={() => {
							openFlowEditor(runnable.path)
						}}
					>
						Edit
					</Button>
					<Button
						variant="default"
						size="xs"
						startIcon={{ icon: Eye }}
						endIcon={{ icon: ExternalLink }}
						target="_blank"
						href="{base}/flows/get/{runnable.path}?workspace={$workspaceStore}"
					>
						Details
					</Button>
				{/if}
			{:else}
				<Button
					size="xs"
					variant="default"
					startIcon={{ icon: Pen }}
					on:click={() => {
						openScriptEditor(runnable.path)
					}}
				>
					Edit
				</Button>
				<Button
					size="xs"
					variant="default"
					startIcon={{ icon: GitFork }}
					on:click={() => {
						fork(runnable.path)
					}}
				>
					Fork
				</Button>
			{/if}
			<Popover
				floatingConfig={{
					middleware: [
					autoPlacement({
						allowedPlacements: [
							'bottom-start',
							'bottom-end',
							'top-start',
							'top-end',
							'top',
							'bottom'
						]
					})
				]
			}}
			closeButton
			contentClasses="block text-primary text-xs p-4 w-[20vh]"
		>
				{#snippet trigger()}
					<Button
						nonCaptureEvent={true}
						btnClasses={'bg-surface text-primay hover:bg-hover'}
						variant="default"
						size="xs">Cache</Button
					>
				{/snippet}
				{#snippet content()}
					{#if runnable.runType == 'flow' && isHubFlowPath(runnable.path)}
						Since this is a reference to a hub flow, cache settings are managed from the flow after
						you fork it into your workspace.
					{:else}
						Since this is a reference to a workspace {runnable.runType}, set the cache in the
						{runnable.runType} settings directly by editing it. The cache will be shared by any app
						or flow that uses this {runnable.runType}.
					{/if}
				{/snippet}
			</Popover>

		<input
			onkeydown={stopPropagation(bubble('keydown'))}
			bind:value={runnable.name}
			placeholder="Background runnable name"
			class="!text-xs !rounded-xs"
		/>
	</div>
	<div class="w-full grow overflow-y-auto">
		{#key `${viewerContext?.stateId ? get(viewerContext.stateId) : 0}-${refreshKey}`}
			{#if notFound}
				<div class="text-red-400">
					{#if runnable.runType == 'flow' && isHubFlowPath(runnable.path)}
						Hub flow not found at {runnable.path}
					{:else}
						{runnable.runType} not found at {runnable.path} in workspace {$workspaceStore}
					{/if}
				</div>
			{:else if runnable.runType == 'script' || runnable.runType == 'hubscript'}
				<div class="border">
					<FlowModuleScript path={runnable.path} />
				</div>
			{:else if runnable.runType == 'flow'}
				{#if isHubFlowPath(runnable.path)}
					{#if hubFlowPreview}
						<div class="flex flex-col flex-1 h-full overflow-auto">
							<FlowGraphViewer
								triggerNode
								provideTriggerContext
								flow={{ ...hubFlowPreview, path: runnable.path }}
							/>
						</div>
					{:else}
						<Skeleton layout={[[40]]} />
					{/if}
				{:else}
					<FlowPathViewer path={runnable.path} />
				{/if}
			{:else}
				Unrecognized runType {runnable.runType}
			{/if}
		{/key}
	</div>
</div>

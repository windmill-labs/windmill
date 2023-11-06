<script lang="ts">
	import { Button, Drawer, DrawerContent, Popup } from '$lib/components/common'
	import FlowModuleScript from '$lib/components/flows/content/FlowModuleScript.svelte'
	import FlowPathViewer from '$lib/components/flows/content/FlowPathViewer.svelte'
	import { emptySchema } from '$lib/utils'
	import { getContext, tick } from 'svelte'
	import type {
		ConnectedAppInput,
		RowAppInput,
		RunnableByPath,
		StaticAppInput,
		UserAppInput
	} from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import { computeFields } from './utils'
	import { inferArgs, loadSchema } from '$lib/infer'
	import RunButton from './RunButton.svelte'
	import { getScriptByPath } from '$lib/scripts'
	import { sendUserToast } from '$lib/toast'
	import { autoPlacement } from '@floating-ui/core'
	import { ExternalLink, Eye, GitFork, Pen, RefreshCw, Trash } from 'lucide-svelte'

	export let runnable: RunnableByPath
	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let id: string

	const { stateId } = getContext<AppViewerContext>('AppViewerContext')

	let drawerFlowViewer: Drawer
	let flowPath: string = ''
	let notFound = false

	const dispatch = createEventDispatcher()

	async function refreshScript(x: RunnableByPath) {
		try {
			let { schema } = await getScriptByPath(x.path)
			if (!deepEqual(x.schema, schema)) {
				x.schema = schema
				fields = computeFields(schema, false, fields)
			}
		} catch (e) {
			notFound = true
			console.error(e)
		}
	}

	async function refreshFlow(x: RunnableByPath) {
		try {
			const { schema } = (await loadSchema($workspaceStore ?? '', x.path, 'flow')) ?? emptySchema()
			if (!deepEqual(x.schema, schema)) {
				x.schema = schema
				fields = computeFields(schema, false, fields)
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
			type: 'runnableByName',
			name: path,
			inlineScript: {
				content,
				language,
				schema,
				path
			}
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
		}
		lastRunnable = runnable
	}
	$: refresh(runnable)
</script>

<Drawer bind:this={drawerFlowViewer} size="1200px">
	<DrawerContent title="Flow {flowPath}" on:close={drawerFlowViewer.closeDrawer}>
		<FlowPathViewer path={flowPath ?? ''} />
	</DrawerContent>
</Drawer>

<div class="p-2 h-full flex flex-col gap-2">
	<div class="flex flex-row-reverse w-full gap-2">
		<RunButton hideShortcut {id} />

		<Button
			variant="border"
			size="xs"
			startIcon={{ icon: RefreshCw }}
			on:click={async () => {
				sendUserToast('Refreshing inputs')
				refresh(runnable)
				$stateId = $stateId + 1
				await tick()
			}}
		>
			Refresh
		</Button>
		<Button
			size="xs"
			variant="border"
			color="red"
			startIcon={{ icon: Trash }}
			on:click={() => {
				dispatch('delete')
			}}
		>
			Clear
		</Button>
		{#if runnable.runType == 'flow'}
			<Button
				variant="border"
				size="xs"
				startIcon={{ icon: Eye }}
				on:click={() => {
					flowPath = runnable.path
					drawerFlowViewer.openDrawer()
				}}
			>
				Expand
			</Button>
			<Button
				variant="border"
				size="xs"
				startIcon={{ icon: Pen }}
				endIcon={{ icon: ExternalLink }}
				target="_blank"
				href="/flows/edit/{runnable.path}?nodraft=true">Edit</Button
			>
			<Button
				variant="border"
				size="xs"
				startIcon={{ icon: Eye }}
				endIcon={{ icon: ExternalLink }}
				target="_blank"
				href="/flows/get/{runnable.path}?workspace={$workspaceStore}"
			>
				Details page
			</Button>
		{:else}
			<Button
				size="xs"
				variant="border"
				startIcon={{ icon: GitFork }}
				on:click={() => {
					fork(runnable.path)
				}}
			>
				Fork
			</Button>
		{/if}
		<Popup
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
		>
			<svelte:fragment slot="button">
				<Button
					nonCaptureEvent={true}
					btnClasses={'bg-surface text-primay hover:bg-hover'}
					color="light"
					variant="border"
					size="xs">Cache</Button
				>
			</svelte:fragment>
			<div class="block text-primary">
				Since this is a reference to a workspace {runnable.runType}, set the cache in the {runnable.runType}
				settings directly by editing it. The cache will be shared by any app or flow that uses this {runnable.runType}.
			</div>
		</Popup>

		<input
			on:keydown|stopPropagation
			bind:value={runnable.name}
			placeholder="Background runnable name"
			class="!text-xs !rounded-xs"
		/>
	</div>
	<div class="w-full">
		{#key $stateId}
			{#if notFound}
				<div class="text-red-400"
					>{runnable.runType} not found at {runnable.path} in workspace {$workspaceStore}</div
				>
			{:else if runnable.runType == 'script' || runnable.runType == 'hubscript'}
				<div class="border">
					<FlowModuleScript path={runnable.path} />
				</div>
			{:else if runnable.runType == 'flow'}
				<FlowPathViewer path={runnable.path} />
			{:else}
				Unrecognized runType {runnable.runType}
			{/if}
		{/key}
	</div>
</div>

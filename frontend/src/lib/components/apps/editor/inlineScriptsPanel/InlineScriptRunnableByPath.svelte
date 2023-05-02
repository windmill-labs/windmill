<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import FlowModuleScript from '$lib/components/flows/content/FlowModuleScript.svelte'
	import FlowPathViewer from '$lib/components/flows/content/FlowPathViewer.svelte'
	import { emptySchema, getScriptByPath, sendUserToast } from '$lib/utils'
	import { getContext } from 'svelte'
	import type {
		ConnectedAppInput,
		RowAppInput,
		RunnableByPath,
		StaticAppInput,
		UserAppInput
	} from '../../inputType'
	import {
		faCodeBranch,
		faExternalLinkAlt,
		faEye,
		faPen,
		faRefresh,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import type { AppViewerContext } from '../../types'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import { computeFields } from './utils'
	import { loadSchema } from '../../utils'
	import { inferArgs } from '$lib/infer'
	import RunButton from './RunButton.svelte'

	export let runnable: RunnableByPath
	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let id: string

	const { stateId } = getContext<AppViewerContext>('AppViewerContext')

	let drawerFlowViewer: Drawer
	let flowPath: string = ''

	const dispatch = createEventDispatcher()

	async function refreshScript(x: RunnableByPath) {
		let { schema } = await getScriptByPath(x.path)
		if (!deepEqual(x.schema, schema)) {
			x.schema = schema
			fields = computeFields(schema, false, fields)
		}
	}

	async function refreshFlow(x: RunnableByPath) {
		const schema = (await loadSchema($workspaceStore ?? '', x.path, 'flow')) ?? emptySchema()
		if (!deepEqual(x.schema, schema)) {
			x.schema = schema
			fields = computeFields(schema, false, fields)
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

	function refresh() {
		if (runnable.runType == 'script') {
			refreshScript(runnable)
		} else if (runnable.runType == 'flow') {
			refreshFlow(runnable)
		}
	}
	$: runnable.runType && refresh()
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
			size="xs"
			variant="border"
			startIcon={{ icon: faCodeBranch }}
			on:click={() => {
				fork(runnable.path)
			}}
		>
			Fork
		</Button>
		<Button
			variant="border"
			size="xs"
			startIcon={{ icon: faRefresh }}
			on:click={() => {
				sendUserToast('Refreshing inputs')
				refresh()
				$stateId = $stateId + 1
			}}
		>
			Refresh
		</Button>
		<Button
			size="xs"
			variant="border"
			color="red"
			startIcon={{ icon: faTrashAlt }}
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
				startIcon={{ icon: faEye }}
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
				startIcon={{ icon: faPen }}
				endIcon={{ icon: faExternalLinkAlt }}
				target="_blank"
				href="/flows/edit/{runnable.path}?nodraft=true">Edit</Button
			>
			<Button
				variant="border"
				size="xs"
				startIcon={{ icon: faEye }}
				endIcon={{ icon: faExternalLinkAlt }}
				target="_blank"
				href="/flows/get/{runnable.path}?workspace={$workspaceStore}"
			>
				Details page
			</Button>
		{/if}
	</div>
	<div class="w-full">
		{#key $stateId}
			{#if runnable.runType == 'script' || runnable.runType == 'hubscript'}
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

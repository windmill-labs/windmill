<script lang="ts">
	import { ScriptService, FlowService, Script } from '$lib/gen'

	import { hubScripts, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'

	import Select from 'svelte-select'

	import { getScriptByPath } from '$lib/utils'
	import RadioButton from './RadioButton.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import HighlightCode from './HighlightCode.svelte'
	import FlowPathViewer from './flows/content/FlowPathViewer.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../defaults'

	export let initialPath: string | undefined = undefined
	export let scriptPath: string | undefined = undefined
	export let allowFlow = false
	export let allowHub = false
	export let itemKind: 'hub' | 'script' | 'flow' = allowHub ? 'hub' : 'script'
	export let kind: Script.kind = Script.kind.SCRIPT
	export let disabled = false

	let items: { value: string; label: string }[] = []
	let drawerViewer: Drawer
	let drawerFlowViewer: Drawer
	let code: string = ''
	let lang: 'deno' | 'python3' | 'go' | 'bash' | undefined

	let options: [[string, any]] = [['Script', 'script']]
	allowHub && options.unshift(['Hub', 'hub'])
	allowFlow && options.push(['Flow', 'flow'])
	const dispatch = createEventDispatcher()

	async function loadItems(): Promise<void> {
		if (itemKind == 'flow') {
			items = (await FlowService.listFlows({ workspace: $workspaceStore! })).map((flow) => ({
				value: flow.path,
				label: `${flow.path}${flow.summary ? ` | ${flow.summary}` : ''}`
			}))
		} else if (itemKind == 'script') {
			items = (await ScriptService.listScripts({ workspace: $workspaceStore!, kind })).map(
				(script) => ({
					value: script.path,
					label: `${script.path}${script.summary ? ` | ${script.summary}` : ''}`
				})
			)
		} else {
			items =
				$hubScripts?.map((x) => ({
					value: x.path,
					label: `${x.path}${x.summary ? ` | ${x.summary}` : ''}`
				})) ?? []
		}
	}

	$: itemKind && $workspaceStore && loadItems()
</script>

<Drawer bind:this={drawerViewer} size="900px">
	<DrawerContent title="Script {scriptPath}" on:close={drawerViewer.closeDrawer}>
		<HighlightCode {code} language={lang} />
	</DrawerContent>
</Drawer>

<Drawer bind:this={drawerFlowViewer} size="900px">
	<DrawerContent title="Flow {scriptPath}" on:close={drawerFlowViewer.closeDrawer}>
		<FlowPathViewer path={scriptPath ?? ''} />
	</DrawerContent>
</Drawer>

<div class="flex flex-row  items-center gap-4 w-full">
	{#if options.length > 1}
		<div class="w-80 mt-1">
			<RadioButton {disabled} bind:value={itemKind} {options} />
		</div>
	{/if}

	{#if disabled}
		<input type="text" value={scriptPath} disabled />
	{:else}
		<Select
			value={items.find((x) => x.value == initialPath)}
			class="grow"
			on:change={() => {
				dispatch('select', { path: scriptPath })
			}}
			bind:justValue={scriptPath}
			{items}
			placeholder="Pick a {itemKind}"
			inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
			containerStyles={SELECT_INPUT_DEFAULT_STYLE.containerStyles}
		/>
	{/if}

	{#if scriptPath !== undefined && scriptPath !== ''}
		{#if itemKind == 'flow'}
			<Button
				color="light"
				size="xs"
				on:click={async () => {
					drawerFlowViewer.openDrawer()
				}}
			>
				Show flow
			</Button>
		{:else}
			<Button
				color="light"
				size="xs"
				on:click={async () => {
					const { language, content } = await getScriptByPath(scriptPath ?? '')
					code = content
					lang = language
					drawerViewer.openDrawer()
				}}
			>
				Show code
			</Button>
		{/if}
	{/if}
</div>

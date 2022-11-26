<script lang="ts">
	import { ScriptService, FlowService, Script } from '$lib/gen'

	import { hubScripts, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'

	import Select from 'svelte-select'

	import { getScriptByPath } from '$lib/utils'
	import RadioButton from './RadioButton.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import HighlightCode from './HighlightCode.svelte'

	export let scriptPath: string | undefined = undefined
	export let allowFlow = false
	export let allowHub = false
	export let itemKind: 'hub' | 'script' | 'flow' = allowHub ? 'hub' : 'script'
	export let kind: Script.kind = Script.kind.SCRIPT

	let items: { value: string; label: string }[] = []
	let drawerViewer: Drawer
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
				label: `${flow.path}${flow.summary ? `| ${flow.summary}` : ''}`
			}))
		} else if (itemKind == 'script') {
			items = (await ScriptService.listScripts({ workspace: $workspaceStore!, kind })).map(
				(script) => ({
					value: script.path,
					label: `${script.path}${script.summary ? `| ${script.summary}` : ''}`
				})
			)
		} else {
			items =
				$hubScripts?.map((x) => ({
					value: x.path,
					label: `${x.path}${x.summary ? `| ${x.summary}` : ''}`
				})) ?? []
		}
	}

	$: $workspaceStore && itemKind && loadItems()

	$: dispatch('select', { path: scriptPath })
</script>

<div class="flex flex-row  items-center gap-4 w-full">
	{#if options.length > 1}
		<div class="w-80">
			<RadioButton bind:value={itemKind} {options} />
		</div>
	{/if}

	<Select class="grow" bind:justValue={scriptPath} {items} placeholder="Pick a {itemKind}" />
	{#if scriptPath !== undefined && scriptPath !== ''}
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
</div>

<Drawer bind:this={drawerViewer}>
	<DrawerContent title="Script {scriptPath}" on:close={drawerViewer.closeDrawer}>
		<HighlightCode {code} language={lang} />
	</DrawerContent>
</Drawer>

<script lang="ts">
	import { ScriptService, FlowService, Script } from '$lib/gen'

	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher, onMount } from 'svelte'

	import Select from './apps/svelte-select/lib/index'

	import { getScriptByPath } from '$lib/scripts'
	import { Button, Drawer, DrawerContent } from './common'
	import HighlightCode from './HighlightCode.svelte'
	import FlowPathViewer from './flows/content/FlowPathViewer.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../defaults'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { Code, Code2, ExternalLink, Pen, RefreshCw } from 'lucide-svelte'
	import type { SupportedLanguage } from '$lib/common'
	import FlowIcon from './home/FlowIcon.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { truncate } from '$lib/utils'

	export let initialPath: string | undefined = undefined
	export let scriptPath: string | undefined = undefined
	export let allowFlow = false
	export let itemKind: 'script' | 'flow' = 'script'
	export let kinds: Script.kind[] = [Script.kind.SCRIPT]
	export let disabled = false
	export let allowRefresh = false

	let items: { value: string; label: string }[] = []
	let drawerViewer: Drawer
	let drawerFlowViewer: Drawer
	let code: string = ''
	let lang: SupportedLanguage | undefined

	let options: [[string, any, any, string | undefined]] = [['Script', 'script', Code2, undefined]]
	allowFlow && options.push(['Flow', 'flow', FlowIcon, '#14b8a6'])
	const dispatch = createEventDispatcher()

	async function loadItems(): Promise<void> {
		if (itemKind == 'flow') {
			items = (await FlowService.listFlows({ workspace: $workspaceStore! })).map((flow) => ({
				value: flow.path,
				label: `${flow.path}${flow.summary ? ` | ${truncate(flow.summary, 20)}` : ''}`
			}))
		} else if (itemKind == 'script') {
			items = (
				await ScriptService.listScripts({ workspace: $workspaceStore!, kinds: kinds.join(',') })
			).map((script) => ({
				value: script.path,
				label: `${script.path}${script.summary ? ` | ${truncate(script.summary, 20)}` : ''}`
			}))
		}
	}

	$: itemKind && $workspaceStore && loadItems()
	let darkMode: boolean = false

	function onThemeChange() {
		if (document.documentElement.classList.contains('dark')) {
			darkMode = true
		} else {
			darkMode = false
		}
	}

	onMount(() => {
		onThemeChange()
	})
</script>

<DarkModeObserver on:change={onThemeChange} />

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

<div class="flex flex-row items-center gap-4 w-full mt-2">
	{#if options.length > 1}
		<div>
			<ToggleButtonGroup bind:selected={itemKind}>
				{#each options as [label, value, icon, selectedColor]}
					<ToggleButton {icon} {disabled} {value} {label} {selectedColor} />
				{/each}
			</ToggleButtonGroup>
		</div>
	{/if}

	{#if disabled}
		<input type="text" value={scriptPath ?? ''} disabled />
	{:else}
		<Select
			value={items?.find((x) => x.value == initialPath)}
			class="grow shrink max-w-full"
			on:change={() => {
				dispatch('select', { path: scriptPath })
			}}
			on:input={(ev) => {
				if (!ev.detail) {
					dispatch('select', { path: undefined })
				}
			}}
			bind:justValue={scriptPath}
			{items}
			placeholder="Pick a {itemKind}"
			inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
			containerStyles={darkMode
				? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
				: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
			portal={false}
		/>
	{/if}

	{#if allowRefresh}
		<Button
			variant="border"
			color="light"
			wrapperClasses="self-stretch"
			on:click={loadItems}
			startIcon={{ icon: RefreshCw }}
			iconOnly
		/>
	{/if}

	{#if scriptPath !== undefined && scriptPath !== ''}
		{#if itemKind == 'flow'}
			<div class="flex gap-2">
				<Button
					endIcon={{ icon: ExternalLink }}
					target="_blank"
					color="light"
					size="xs"
					variant="border"
					href="/flows/edit/{scriptPath}">Edit</Button
				>
				<Button
					color="light"
					size="xs"
					variant="border"
					on:click={async () => {
						drawerFlowViewer.openDrawer()
					}}
				>
					View
				</Button>
			</div>
		{:else}
			<div class="flex gap-2">
				<Button
					startIcon={{ icon: Pen }}
					target="_blank"
					color="light"
					size="xs"
					href="/scripts/edit/{scriptPath}"
					variant="border"
				>
					Edit
				</Button>
				<Button
					color="light"
					size="xs"
					variant="border"
					startIcon={{ icon: Code }}
					on:click={async () => {
						const { language, content } = await getScriptByPath(scriptPath ?? '')
						code = content
						lang = language
						drawerViewer.openDrawer()
					}}
				>
					View
				</Button>
			</div>
		{/if}
	{/if}
</div>

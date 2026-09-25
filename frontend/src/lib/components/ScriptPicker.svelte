<script lang="ts">
	import { ScriptService, FlowService, type Script, AppService } from '$lib/gen'

	import { base } from '$lib/base'
	import { createEventDispatcher, untrack } from 'svelte'

	import Select from './select/Select.svelte'

	import { getScriptByPath } from '$lib/scripts'
	import { Button, Drawer, DrawerContent } from './common'
	import HighlightCode from './HighlightCode.svelte'
	import FlowPathViewer from './flows/content/FlowPathViewer.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { Code, Code2, ExternalLink, Globe2, Pen, RefreshCw } from 'lucide-svelte'
	import PickHubScript from './flows/pickers/PickHubScript.svelte'
	import { disableHubStore } from '$lib/stores'
	import type { SupportedLanguage } from '$lib/common'
	import FlowIcon from './home/FlowIcon.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { truncate } from '$lib/utils'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		initialPath?: string | undefined
		scriptPath?: string | undefined
		allowFlow?: boolean
		itemKind?: 'script' | 'flow' | 'app'
		kinds?: Script['kind'][]
		disabled?: boolean
		allowRefresh?: boolean
		allowEdit?: boolean
		allowView?: boolean
		clearable?: boolean
		/** Offer picking a script from the Hub; the picked path is `hub/...`. Browses plain
		 * scripts only, whatever `kinds` is. */
		allowHub?: boolean
		/** Workspace to list runnables from. Defaults to the operating workspace (see
		 * `useOperatingWorkspace`). */
		workspace?: string
	}

	let {
		initialPath = undefined,
		scriptPath = $bindable(undefined),
		allowFlow = false,
		itemKind = $bindable('script'),
		kinds = ['script'],
		disabled = false,
		allowRefresh = false,
		allowEdit = true,
		allowView = true,
		clearable = false,
		allowHub = false,
		workspace = undefined
	}: Props = $props()

	let isHubPath = $derived(itemKind == 'script' && !!scriptPath?.startsWith('hub/'))
	let drawerHub: Drawer | undefined = $state()
	let filterText = $state('')
	let hubFilter = $state('')

	let effectiveWorkspace = $derived(workspace ?? $operatingWorkspace)
	// Edit/View routes open in the workspace listed here, not wherever the tab lands.
	let wsParam = $derived(
		effectiveWorkspace ? `?workspace=${encodeURIComponent(effectiveWorkspace)}` : ''
	)

	let items: { value: string; label: string }[] = $state([])
	let drawerViewer: Drawer | undefined = $state()
	let drawerFlowViewer: Drawer | undefined = $state()
	let code: string = $state('')
	let lang: SupportedLanguage | undefined = $state()

	let options: [[string, any, any, string | undefined]] = [['Script', 'script', Code2, undefined]]
	untrack(() => allowFlow) && options.push(['Flow', 'flow', FlowIcon, '#14b8a6'])
	const dispatch = createEventDispatcher()

	async function loadItems(): Promise<void> {
		if (itemKind == 'flow') {
			items = (
				await FlowService.listFlows({ workspace: effectiveWorkspace!, withoutDescription: true })
			).map((flow) => ({
				value: flow.path,
				label: `${flow.path}${flow.summary ? ` | ${truncate(flow.summary, 20)}` : ''}`,
				withoutDescription: true
			}))
		} else if (itemKind == 'script') {
			items = (
				await ScriptService.listScripts({
					workspace: effectiveWorkspace!,
					kinds: kinds.join(','),
					withoutDescription: true
				})
			).map((script) => ({
				value: script.path,
				label: `${script.path}${script.summary ? ` | ${truncate(script.summary, 20)}` : ''}`
			}))
		} else if (itemKind == 'app') {
			items = (await AppService.listApps({ workspace: effectiveWorkspace! })).map((app) => ({
				value: app.path,
				label: `${app.path}${app.summary ? ` | ${truncate(app.summary, 20)}` : ''}`
			}))
		}
	}

	$effect(() => {
		itemKind && effectiveWorkspace && untrack(() => loadItems())
	})
	let darkMode: boolean = $state(false)
</script>

<DarkModeObserver bind:darkMode />

<Drawer bind:this={drawerViewer} size="900px">
	<DrawerContent title="Script {scriptPath}" on:close={drawerViewer.closeDrawer}>
		<HighlightCode {code} language={lang} />
	</DrawerContent>
</Drawer>

<Drawer bind:this={drawerFlowViewer} size="900px">
	<DrawerContent title="Flow {scriptPath}" on:close={drawerFlowViewer.closeDrawer}>
		<FlowPathViewer path={scriptPath ?? ''} workspace={effectiveWorkspace} />
	</DrawerContent>
</Drawer>

{#if allowHub}
	<Drawer bind:this={drawerHub} size="900px">
		<DrawerContent title="Pick a Hub script" on:close={drawerHub.closeDrawer}>
			<PickHubScript
				bind:filter={hubFilter}
				on:pick={(e) => {
					scriptPath = e.detail.path
					dispatch('select', { path: e.detail.path, itemKind })
					drawerHub?.closeDrawer()
				}}
			/>
		</DrawerContent>
	</Drawer>
{/if}

<div class="flex flex-row items-center gap-1 w-full">
	{#if options.length > 1}
		<div>
			<ToggleButtonGroup
				bind:selected={itemKind}
				on:selected={() => {
					scriptPath = ''
				}}
			>
				{#snippet children({ item })}
					{#each options as [label, value, icon, selectedColor]}
						<ToggleButton {icon} {disabled} {value} {label} {selectedColor} {item} />
					{/each}
				{/snippet}
			</ToggleButtonGroup>
		</div>
	{/if}

	{#if disabled}
		<input type="text" value={scriptPath ?? initialPath ?? ''} disabled />
	{:else}
		<Select
			bind:value={
				() => (scriptPath ?? initialPath) || undefined,
				(path) => {
					scriptPath = path
					dispatch('select', { path, itemKind })
				}
			}
			class="grow shrink max-w-full"
			items={isHubPath ? [{ value: scriptPath!, label: scriptPath! }, ...items] : items}
			{clearable}
			bind:filterText
			placeholder="Pick {itemKind === 'app' ? 'an' : 'a'} {itemKind}"
			bottomSnippet={allowHub && itemKind == 'script' && !$disableHubStore ? hubHint : undefined}
		/>
	{/if}

	{#if allowRefresh}
		<Button
			variant="subtle"
			unifiedSize="md"
			on:click={loadItems}
			startIcon={{ icon: RefreshCw }}
			iconOnly
		/>
	{/if}

	{#if scriptPath !== undefined && scriptPath !== ''}
		{#if itemKind == 'flow'}
			<div class="flex gap-1">
				{#if allowEdit}
					<Button
						endIcon={{ icon: ExternalLink }}
						target="_blank"
						variant="default"
						size="xs"
						href="{base}/flows/edit/{scriptPath}{wsParam}">Edit</Button
					>
				{/if}
				{#if allowView}
					<Button
						variant="default"
						size="xs"
						on:click={async () => {
							drawerFlowViewer?.openDrawer()
						}}
					>
						View
					</Button>
				{/if}
			</div>
		{:else if itemKind == 'app'}
			<div class="flex gap-2">
				{#if allowEdit}
					<Button
						startIcon={{ icon: Pen }}
						target="_blank"
						variant="default"
						size="xs"
						href="{base}/apps/edit/{scriptPath}{wsParam}"
					>
						Edit
					</Button>
				{/if}
				{#if allowView}
					<Button
						variant="default"
						size="xs"
						target="_blank"
						startIcon={{ icon: Code }}
						href="{base}/apps/get/{scriptPath}{wsParam}"
					>
						View
					</Button>
				{/if}
			</div>
		{:else}
			<div class="flex gap-2">
				{#if allowEdit && !isHubPath}
					<Button
						startIcon={{ icon: Pen }}
						target="_blank"
						variant="default"
						size="xs"
						href="{base}/scripts/edit/{scriptPath}{wsParam}"
					>
						Edit
					</Button>
				{/if}
				{#if allowView}
					<Button
						variant="default"
						size="xs"
						startIcon={{ icon: Code }}
						on:click={async () => {
							const { language, content } = await getScriptByPath(
								scriptPath ?? '',
								effectiveWorkspace
							)
							code = content
							lang = language
							drawerViewer?.openDrawer()
						}}
					>
						View
					</Button>
				{/if}
			</div>
		{/if}
	{/if}
</div>

{#snippet hubHint({ close }: { close: () => void })}
	<button
		class="sticky py-2 px-4 w-full text-left text-xs font-medium hover:bg-surface-hover flex items-center justify-center gap-2 border-t border-border-light"
		onclick={() => {
			// Read before close(): Select clears its filter text when the list closes.
			hubFilter = filterText
			close()
			drawerHub?.openDrawer()
		}}
	>
		<Globe2 class="inline" size={16} />
		Browse Hub scripts
	</button>
{/snippet}

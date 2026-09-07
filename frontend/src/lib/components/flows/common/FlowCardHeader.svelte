<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import MetadataGen from '$lib/components/copilot/MetadataGen.svelte'
	import IconedPath from '$lib/components/IconedPath.svelte'
	import { ScriptService, type FlowModuleValue } from '$lib/gen'
	import {
		ArrowUpCircle,
		Flag,
		GitFork,
		Lock,
		Pen,
		RefreshCw,
		Settings,
		Unlock
	} from 'lucide-svelte'
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import FlowPanelChrome from './FlowPanelChrome.svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { hubBaseUrlStore, workspaceStore } from '$lib/stores'
	import { DEFAULT_HUB_BASE_URL, PRIVATE_HUB_MIN_VERSION } from '$lib/hub'
	import { getLatestHashForScript } from '$lib/scripts'
	import { sendUserToast, type Item } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { getToolNameError } from '$lib/components/flows/agentToolUtils'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import autosize from '$lib/autosize'

	interface Props {
		flowModuleValue?: FlowModuleValue | undefined
		title?: string | undefined
		summary?: string | undefined
		description?: string | undefined
		/** Static one-line explanation of what this kind of step does. Not the editable
		 *  `description`, which is the AI-tool prompt the user writes. */
		subtitle?: string | undefined
		subtitleDocLink?: string | undefined
		children?: import('svelte').Snippet
		action?: import('svelte').Snippet
		isAgentTool?: boolean
		siblingToolNames?: string[]
	}

	let {
		flowModuleValue = undefined,
		title = undefined,
		summary = $bindable(undefined),
		description = $bindable(undefined),
		subtitle = undefined,
		subtitleDocLink = undefined,
		children,
		action,
		isAgentTool = false,
		siblingToolNames = undefined
	}: Props = $props()

	let toolNameError = $derived(
		isAgentTool ? getToolNameError(summary ?? '', undefined, siblingToolNames) : undefined
	)

	const dispatch = createEventDispatcher()
	const customUi: FlowBuilderWhitelabelCustomUi | undefined = getContext('customUi')
	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	const { scriptEditorDrawer, workspaceScriptSettingsDrawer } = flowEditorContext

	let opWs = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)
	const scriptPath = $derived(flowModuleValue?.type === 'script' ? flowModuleValue.path : undefined)
	const pinnedHash = $derived(flowModuleValue?.type === 'script' ? flowModuleValue.hash : undefined)
	const isHub = $derived(scriptPath?.startsWith('hub/') ?? false)
	// Version id out of a hub path: hub/{version_id}/{app}/{summary}
	const hubVersionId = $derived(isHub ? scriptPath?.split('/')[1] : undefined)

	let latestHash: string | undefined = $state(undefined)
	$effect(() => {
		const path = scriptPath
		if (!opWs || !path || isHub) return
		untrack(async () => {
			latestHash = (await ScriptService.getScriptByPath({ workspace: opWs, path })).hash
		})
	})

	function reportIssue() {
		const targetHubBaseUrl =
			Number(hubVersionId) < PRIVATE_HUB_MIN_VERSION ? DEFAULT_HUB_BASE_URL : $hubBaseUrlStore
		window.open(
			`${targetHubBaseUrl}/from_version/${hubVersionId}?report_issue=${hubVersionId}`,
			'_blank'
		)
	}

	// Every one of these acts on the referenced script rather than the step, so they share
	// a single menu instead of a row of icon buttons.
	const scriptItems: Item[] = $derived.by(() => {
		if (flowModuleValue?.type !== 'script') return []
		const items: Item[] = []
		if (!isHub && customUi?.scriptEdit != false) {
			items.push({
				displayName: "Edit the script's code",
				icon: Pen,
				disabled: pinnedHash != undefined,
				tooltip: pinnedHash != undefined ? 'Unlock the hash to edit' : undefined,
				action: async () => {
					if (flowModuleValue?.type !== 'script') return
					const hash =
						flowModuleValue.hash ?? (await getLatestHashForScript(flowModuleValue.path, opWs))
					// Same reason the settings item below is gated: the local-dev editors publish
					// the context store but never render the drawer, so an unmounted one makes
					// this a no-op — and a no-op must not be counted as an editor open.
					const drawer = $scriptEditorDrawer
					if (!drawer) return
					logFeatureUsage('flow_step', 'script_edit', { key: 'opened' })
					// The drawer only runs this callback once a new version is deployed, so it is
					// what separates opening the editor from actually editing the script here.
					drawer.openDrawer(hash, () => {
						logFeatureUsage('flow_step', 'script_edit', { key: 'saved' })
						dispatch('reload')
						sendUserToast('Script has been updated')
					})
				}
			})
			// Only when the settings drawer is actually mounted (not in the local-dev
			// editors, which provide the context store but never render it).
			if ($workspaceScriptSettingsDrawer) {
				items.push({
					displayName: 'Runtime settings',
					icon: Settings,
					disabled: pinnedHash != undefined,
					tooltip: 'Concurrency, cache, timeout, …',
					action: () => {
						if (flowModuleValue?.type !== 'script') return
						$workspaceScriptSettingsDrawer?.openDrawer(
							flowModuleValue.path,
							flowModuleValue.hash,
							() => dispatch('reload')
						)
					}
				})
			}
		}
		if (customUi?.scriptFork != false) {
			items.push({
				displayName: 'Fork into an inline script',
				icon: GitFork,
				action: () => dispatch('fork')
			})
		}
		if (pinnedHash) {
			if (latestHash && latestHash !== pinnedHash) {
				items.push({
					displayName: 'Update to latest hash',
					icon: ArrowUpCircle,
					separatorTop: items.length > 0,
					action: () => {
						dispatch('setHash', latestHash)
						dispatch('reload')
					}
				})
			}
			items.push({
				displayName: 'Unlock hash',
				icon: Unlock,
				tooltip: 'Always use the latest deployed version at that path',
				separatorTop: items.length > 0 && !items.at(-1)?.separatorTop,
				action: () => dispatch('setHash', undefined)
			})
		} else if (latestHash) {
			items.push({
				displayName: 'Lock hash',
				icon: Lock,
				tooltip: 'Always use this specific version',
				separatorTop: items.length > 0,
				action: () => dispatch('setHash', latestHash)
			})
			items.push({
				displayName: 'Reload latest hash',
				icon: RefreshCw,
				action: () => dispatch('reload')
			})
		}
		if (hubVersionId) {
			items.push({
				displayName: 'Report issue',
				icon: Flag,
				separatorTop: items.length > 0,
				action: reportIssue
			})
		}
		return items
	})
</script>

<div class="flex flex-col gap-1 px-4 py-2">
	<div
		class="overflow-x-auto scrollbar-hidden flex items-center justify-between flex-nowrap w-full"
	>
		{#if flowModuleValue}
			<span class="mr-4 min-w-0 flex-1 text-sm">
				<div class="flex min-w-0 items-center space-x-2">
					{#if flowModuleValue.type === 'identity'}
						<span class="font-bold text-xs">Identity (input copied to output)</span>
					{:else if flowModuleValue.type === 'rawscript'}
						<div class="mx-0.5">
							<LanguageIcon lang={flowModuleValue.language} width={20} height={20} />
						</div>
						<MetadataGen
							bind:content={summary}
							promptConfigName={isAgentTool ? 'agentToolFunctionName' : 'summary'}
							code={flowModuleValue.content}
							class="w-full"
							elementProps={{
								placeholder: isAgentTool ? 'Tool name' : 'Summary'
							}}
							hideError={isAgentTool}
							{siblingToolNames}
						/>
					{:else if flowModuleValue.type === 'script' && 'path' in flowModuleValue && flowModuleValue.path}
						<IconedPath
							path={flowModuleValue.path}
							hash={flowModuleValue.hash}
							class="!w-auto shrink min-w-0"
						/>
						{#if scriptItems.length > 0}
							<DropdownV2 size="sm" placement="bottom-end" items={scriptItems} />
						{/if}

						<div class="flex min-w-[8rem] flex-1 flex-col">
							<input
								bind:value={summary}
								placeholder={isAgentTool ? 'Tool name' : 'Summary'}
								class={twMerge('w-full grow', toolNameError && '!border-red-400')}
							/>
							{#if toolNameError && !isAgentTool}
								<p class="text-3xs text-red-400 leading-tight mt-0.5">{toolNameError}</p>
							{/if}
						</div>
					{:else if flowModuleValue.type === 'flow'}
						<Badge color="indigo" capitalize>flow</Badge>
						<input bind:value={summary} placeholder="Summary" class="w-full grow" />
					{:else if flowModuleValue.type === 'aiagent'}
						<Badge color="indigo">AI Agent</Badge>
						<input bind:value={summary} placeholder="Summary" class="w-full grow" />
					{/if}
				</div>
			</span>
		{/if}
		{#if title}
			<!-- Absorbs the free space so the actions stay together on the right: with
			     justify-between alone, adding the detach button centres them. -->
			<div class="mr-auto truncate pr-2 text-sm font-semibold text-emphasis">{title}</div>
		{/if}
		{@render children?.()}
		{@render action?.()}
		<FlowPanelChrome />
	</div>
	{#if subtitle}
		<p class="text-xs leading-snug text-tertiary">
			{subtitle}
			{#if subtitleDocLink}
				<a
					href={subtitleDocLink}
					target="_blank"
					rel="noreferrer"
					class="text-blue-500 hover:underline">Docs</a
				>
			{/if}
		</p>
	{/if}
	{#if isAgentTool}
		{#if toolNameError}
			<p class="text-3xs text-red-400 leading-tight w-full">{toolNameError}</p>
		{/if}
		<textarea
			rows="1"
			use:autosize={{ minHeight: 0 }}
			bind:value={description}
			maxlength={3000}
			placeholder="Tool description (optional): tells the AI when and how to use this tool"
			class="w-full text-xs resize-none"
		></textarea>
	{/if}
</div>

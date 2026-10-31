<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Version from './Version.svelte'
	import DarkModeToggle from './sidebar/DarkModeToggle.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import TokensTable from './settings/TokensTable.svelte'
	import { createEventDispatcher, tick } from 'svelte'
	import CreateToken from './settings/CreateToken.svelte'
	import UserInfoSettings from './settings/UserInfoSettings.svelte'
	import AIUserSettings from './settings/AIUserSettings.svelte'
	import AiUsagePanel from './workspaceSettings/AiUsagePanel.svelte'
	import { copilotInfo, copilotWorkspace } from '$lib/aiStore'
	import {
		getDarkModeVariant,
		setDarkModeVariant,
		type DarkModeVariant
	} from '$lib/darkModeVariant'

	interface Props {
		scopes?: string[] | undefined
		newTokenLabel?: string | undefined
		newTokenWorkspace?: string | undefined
		newToken?: string | undefined
		showMcpMode?: boolean
		disableChatOffset?: boolean
	}

	let {
		scopes = undefined,
		newTokenLabel = undefined,
		newTokenWorkspace = undefined,
		newToken = $bindable(undefined),
		showMcpMode = false,
		disableChatOffset = false
	}: Props = $props()

	let drawer: Drawer | undefined = $state()
	let mcpSection: HTMLDivElement | undefined = $state()
	let tokensTable: TokensTable | undefined = $state()
	let darkVariant = $state<DarkModeVariant>(getDarkModeVariant())

	const dispatch = createEventDispatcher()

	export async function openDrawer(mcpMode: boolean = false) {
		drawer?.openDrawer()
		if (mcpMode) {
			await tick()
			mcpSection?.scrollIntoView({ block: 'start' })
		}
	}

	export function closeDrawer() {
		drawer?.closeDrawer()
		removeHash()
	}

	function removeHash() {
		window.location.hash = ''
	}

	function handleTokenCreated(token: string) {
		newToken = token
		dispatch('tokenCreated', token)
	}
</script>

<Drawer
	bind:this={drawer}
	size="900px"
	on:open={() => (darkVariant = getDarkModeVariant())}
	on:close={removeHash}
	{disableChatOffset}
>
	<DrawerContent title="User settings" on:close={closeDrawer}>
		<div class="flex flex-col gap-6 pb-8">
			{#if scopes == undefined}
				<div
					class="flex flex-row justify-between items-start gap-2 border border-border-light p-4 rounded-md"
				>
					<div class="flex flex-col gap-2">
						<div class="font-semibold text-emphasis text-xs flex items-center">
							Theme <DarkModeToggle forcedDarkMode={false} />
						</div>
						<div class="flex items-center gap-2">
							<span class="text-xs text-secondary">Dark variant</span>
							<ToggleButtonGroup
								selected={darkVariant}
								class="w-fit"
								onSelected={(v) => {
									darkVariant = v
									setDarkModeVariant(v)
								}}
							>
								{#snippet children({ item })}
									<ToggleButton value="default" label="Default" size="sm" {item} />
									<ToggleButton value="github" label="GitHub" size="sm" {item} />
								{/snippet}
							</ToggleButtonGroup>
						</div>
					</div>
					<div class="text-xs text-emphasis flex-col flex">
						Windmill <Version />
					</div>
				</div>
				<div class="grid grid-cols-1 lg:grid-cols-2 w-full gap-4">
					<div class="min-w-0">
						<UserInfoSettings />
					</div>
					<div class="min-w-0">
						<AIUserSettings />
					</div>
				</div>
				<!-- Keyed on the workspace `copilotInfo` reflects, not on `$workspaceStore`:
				     a session acting on another workspace loads that workspace's AI config
				     while navigation stays put, and pricing usage from one workspace with
				     another's rates would silently misstate it. -->
				{#if $copilotWorkspace}
					<AiUsagePanel
						workspace={$copilotWorkspace}
						modelPricing={$copilotInfo.modelPricing ?? {}}
						scope="self"
					/>
				{/if}
			{/if}

			{#if showMcpMode}
				<div bind:this={mcpSection} class="flex flex-col p-4 border border-border-light rounded-md">
					<h2 class="text-emphasis text-sm font-semibold mb-1">
						Build and run from your AI client
					</h2>
					<div class="text-xs text-secondary mb-2">
						Connect Claude, ChatGPT or Cursor to Windmill over MCP: build scripts, flows and apps,
						run your automations and check their runs, from your chat.
					</div>
					<CreateToken mcpOnly onTokenCreated={() => tokensTable?.listTokens()} />
				</div>
			{/if}

			<div class="grow min-h-0">
				<TokensTable
					bind:this={tokensTable}
					defaultNewTokenLabel={newTokenLabel}
					defaultNewTokenWorkspace={newTokenWorkspace}
					{scopes}
					onTokenCreated={handleTokenCreated}
				/>
			</div>
		</div>
	</DrawerContent>
</Drawer>

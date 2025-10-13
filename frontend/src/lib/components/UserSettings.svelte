<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Version from './Version.svelte'
	import DarkModeToggle from './sidebar/DarkModeToggle.svelte'
	import TokensTable from './settings/TokensTable.svelte'
	import { createEventDispatcher } from 'svelte'
	import UserInfoSettings from './settings/UserInfoSettings.svelte'
	import AIUserSettings from './settings/AIUserSettings.svelte'

	interface Props {
		scopes?: string[] | undefined
		newTokenLabel?: string | undefined
		newTokenWorkspace?: string | undefined
		newToken?: string | undefined
		showMcpMode?: boolean
	}

	let {
		scopes = undefined,
		newTokenLabel = undefined,
		newTokenWorkspace = undefined,
		newToken = $bindable(undefined),
		showMcpMode = false
	}: Props = $props()

	let drawer: Drawer | undefined = $state()
	let openWithMcpMode = $state(false)

	const dispatch = createEventDispatcher()

	export function openDrawer(mcpMode: boolean = false) {
		openWithMcpMode = mcpMode
		drawer?.openDrawer()
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

<Drawer bind:this={drawer} size="900px" on:close={removeHash}>
	<DrawerContent title="User Settings" on:close={closeDrawer}>
		<div class="flex flex-col h-full gap-6">
			{#if scopes == undefined}
				<div class="flex flex-col gap-2 border border-border-light p-4">
					<div class="text-xs text-emphasis flex-col flex">
						Windmill <Version />
					</div>
					<div class="font-medium text-emphasis text-xs flex items-baseline">
						Theme <DarkModeToggle forcedDarkMode={false} />
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
			{/if}

			<TokensTable
				{showMcpMode}
				{openWithMcpMode}
				defaultNewTokenLabel={newTokenLabel}
				defaultNewTokenWorkspace={newTokenWorkspace}
				{scopes}
				onTokenCreated={handleTokenCreated}
			/>
		</div>
	</DrawerContent>
</Drawer>

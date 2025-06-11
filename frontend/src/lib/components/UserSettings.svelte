<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Version from './Version.svelte'
	import DarkModeToggle from './sidebar/DarkModeToggle.svelte'
	import TokensTable from './settings/TokensTable.svelte'
	import { createEventDispatcher } from 'svelte'
	import UserInfoSettings from './settings/UserInfoSettings.svelte'
	import AIUserSettings from './settings/AIUserSettings.svelte'

	export let scopes: string[] | undefined = undefined
	export let newTokenLabel: string | undefined = undefined
	export let newTokenWorkspace: string | undefined = undefined
	export let newToken: string | undefined = undefined
	export let showMcpMode: boolean = false

	let drawer: Drawer
	let openWithMcpMode = false

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

	function handleTokenCreated(newToken: string) {
		dispatch('tokenCreated', newToken)
	}
</script>

<Drawer bind:this={drawer} size="800px" on:close={removeHash}>
	<DrawerContent title="User Settings" on:close={closeDrawer}>
		<div class="flex flex-col h-full">
			{#if scopes == undefined}
				<div class="text-xs pt-1 pb-2 text-tertiary flex-col flex">
					Windmill <Version />
				</div>
				<div class="font-semibold flex items-baseline">
					Theme: <DarkModeToggle forcedDarkMode={false} />
				</div>
				<div class="flex flex-wrap md:flex-nowrap w-full md:gap-20 gap-10 mt-2">
					<div class="md:w-[45%]">
						<UserInfoSettings />
					</div>
					<div class="md:w-[45%]">
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

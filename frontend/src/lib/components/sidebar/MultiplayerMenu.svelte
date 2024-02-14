<script lang="ts">
	import { enterpriseLicense, userStore, workspaceStore, awarenessStore } from '$lib/stores'
	import { BROWSER } from 'esm-env'

	import { WebsocketProvider } from 'y-websocket'
	import * as Y from 'yjs'
	import type { Awareness } from 'y-protocols/awareness'
	import { page } from '$app/stores'
	import { slide } from 'svelte/transition'

	const wsProtocol = BROWSER && window.location.protocol == 'https:' ? 'wss' : 'ws'

	let awareness: Awareness | undefined = undefined
	let wsProvider: WebsocketProvider | undefined = undefined

	let connected = false
	function connectWorkspace(workspace: string) {
		const ydoc = new Y.Doc()
		wsProvider = new WebsocketProvider(
			`${wsProtocol}://${window.location.host}/ws_mp/`,
			workspace,
			ydoc
		)
		wsProvider.on('sync', (isSynced: boolean) => {
			connected = true
		})

		awareness = wsProvider.awareness

		awareness?.setLocalState({
			name: $userStore?.username,
			url: $page.url.pathname
		})

		function setPeers() {
			if (!awareness) return
			$awarenessStore = Object.fromEntries(
				Array.from(awareness.getStates().values()).map((x) => [x.name, x.url])
			)
		}

		setPeers()
		// You can observe when a user updates their awareness information
		awareness?.on('change', (changes) => {
			setPeers()
		})
	}
	$: awareness?.setLocalState({
		name: $userStore?.username,
		url: $page.url.pathname
	})
	$: $enterpriseLicense && $workspaceStore && connectWorkspace($workspaceStore)

	function showActivity(url: string) {
		if (url.startsWith('/scripts/add')) {
			return 'Creating a script'
		} else if (url.startsWith('/scripts/edit')) {
			return 'Editing a script'
		} else if (url.startsWith('/scripts/get')) {
			return 'Viewing a script'
		} else if (url.startsWith('/flows/add')) {
			return 'Creating a flow'
		} else if (url.startsWith('/flows/edit')) {
			return 'Editing a flow'
		} else if (url.startsWith('/flows/get')) {
			return 'Viewing a flow'
		} else if (url.startsWith('/apps/add')) {
			return 'Creating an app'
		} else if (url.startsWith('/apps/edit')) {
			return 'Editing an app'
		} else if (url.startsWith('/runs')) {
			return 'Exploring runs'
		} else if (url.startsWith('/variables')) {
			return 'Exploring variables'
		} else if (url.startsWith('/resources')) {
			return 'Exploring runs'
		} else if (url == '/') {
			return 'On the home page'
		} else {
			return ''
		}
	}
</script>

{#if connected}
	<div class="divide-gray-100 border-t" role="none">
		<div class="px-2.5 text-xs font-semibold mt-1">Live activity</div>
		<div class="py-1 flex flex-col gap-y-1 max-h-48 overflow-auto" transition:slide>
			{#each Object.entries($awarenessStore ?? {}) as [user, url]}
				<div class="inline-flex gap-2 px-2 items-center">
					<span
						class="inline-flex h-6 w-6 px-1 items-center justify-center rounded-full ring-2 ring-white bg-gray-600"
						title={user}
					>
						<span class="text-sm font-medium leading-none text-white"
							>{user.substring(0, 2).toLocaleUpperCase()}</span
						>
					</span>
					<div class="flex flex-col">
						<span class="text-sm text-primary truncate">{user}</span>
						<span class="text-xs text-tertiary truncate">{showActivity(url)}</span>
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}

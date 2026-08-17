<script lang="ts">
	import { run } from 'svelte/legacy'
	import { onDestroy } from 'svelte'

	import { enterpriseLicense, userStore, workspaceStore, awarenessStore } from '$lib/stores'

	import { WebsocketProvider } from 'y-websocket'
	import * as Y from 'yjs'
	import type { Awareness } from 'y-protocols/awareness'
	import { page } from '$app/stores'
	import { slide } from 'svelte/transition'
	import { buildWsUrl } from '$lib/wsUrl'
	import { signMultiplayerRequest } from '$lib/components/debug'

	let awareness: Awareness | undefined = $state(undefined)
	let wsProvider: WebsocketProvider | undefined = undefined

	let connected = $state(false)

	function updateLocalState() {
		if (!awareness || !$userStore?.username) return
		awareness.setLocalState({
			name: $userStore.username,
			url: $page.url.pathname
		})
	}
	function disconnectWorkspace() {
		if (wsProvider) {
			wsProvider.destroy()
			wsProvider = undefined
		}
		connected = false
		awareness = undefined
	}
	async function connectWorkspace(workspace: string) {
		disconnectWorkspace()

		let token: string | undefined
		try {
			token = await signMultiplayerRequest(workspace)
		} catch (e) {
			console.error('Failed to sign multiplayer request:', e)
			return
		}

		const ydoc = new Y.Doc()
		wsProvider = new WebsocketProvider(buildWsUrl('/ws_mp/'), workspace, ydoc, {
			params: { token }
		})
		wsProvider.on('sync', (isSynced: boolean) => {
			connected = true
		})

		awareness = wsProvider.awareness

		updateLocalState()

		function setPeers() {
			if (!awareness) return
			const states = Array.from(awareness.getStates().values()).filter((x) => x.name)
			const peerMap: Record<string, string> = {}
			for (const state of states) {
				if (state.name === $userStore?.username) {
					// For current user, always use this tab's URL to avoid multi-tab flickering
					peerMap[state.name] = $page.url.pathname
				} else if (!Object.prototype.hasOwnProperty.call(peerMap, state.name)) {
					// For other users, keep first seen URL per username (stable dedup)
					peerMap[state.name] = state.url
				}
			}
			$awarenessStore = peerMap
		}

		setPeers()
		// You can observe when a user updates their awareness information
		awareness?.on('change', (changes) => {
			setPeers()
		})
	}
	run(() => {
		updateLocalState()
	})
	run(() => {
		$enterpriseLicense && $workspaceStore && connectWorkspace($workspaceStore)
	})

	onDestroy(() => {
		disconnectWorkspace()
	})

	let peers = $derived(
		Object.entries($awarenessStore ?? {}).filter(
			([user]) => user && user !== 'undefined' && user !== 'null'
		)
	)

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
		<div class="px-2 text-xs text-secondary font-normal mt-1">Live activity</div>
		<div class="py-1 flex flex-col gap-y-1 max-h-48 overflow-auto" transition:slide>
			{#each peers as [user, url] (user)}
				<div class="inline-flex gap-2 px-2 items-center">
					<span
						class="inline-flex h-6 w-6 px-1 items-center justify-center rounded-full ring-2 ring-white bg-gray-600"
						title={user}
					>
						<span class="text-sm font-medium leading-none text-white"
							>{user?.substring(0, 2).toLocaleUpperCase()}</span
						>
					</span>
					<div class="flex flex-col">
						<span class="text-xs text-primary truncate">{user}</span>
						<span class="text-2xs text-secondary truncate">{showActivity(url ?? '')}</span>
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}

<script lang="ts">
	import { enterpriseLicense, userStore, workspaceStore, awarenessStore } from '$lib/stores'
	import { classNames } from '$lib/utils'
	import { Users } from 'lucide-svelte'
	import { BROWSER } from 'esm-env'

	import Menu from '../common/menu/Menu.svelte'
	import { WebsocketProvider } from 'y-websocket'
	import * as Y from 'yjs'
	import type { Awareness } from 'y-protocols/awareness'
	import { page } from '$app/stores'

	export let isCollapsed: boolean = false
	const wsProtocol = BROWSER && window.location.protocol == 'https:' ? 'wss' : 'ws'

	let awareness: Awareness | undefined = undefined
	let wsProvider: WebsocketProvider | undefined = undefined

	function connectWorkspace(workspace: string) {
		const ydoc = new Y.Doc()
		wsProvider = new WebsocketProvider(
			`${wsProtocol}://${window.location.host}/ws_mp/`,
			workspace,
			ydoc
		)
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
		awareness.on('change', (changes) => {
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

<Menu placement="bottom-start">
	<button
		slot="trigger"
		type="button"
		class={classNames(
			'group w-full flex items-center text-white hover:bg-gray-50 hover:text-gray-900 focus:ring-4 focus:outline-none focus:ring-gray-300 px-2 py-2 text-sm font-medium rounded-md h-8 '
		)}
	>
		<div class="center-center mr-2">
			<Users size={16} />
		</div>

		{#if !isCollapsed}
			<span class={classNames('whitespace-pre truncate')}>Live</span>
		{/if}
	</button>

	<div class="divide-y divide-gray-100" role="none">
		<div class="py-1 flex flex-col gap-y-1">
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
						<span class="text-sm text-gray-900 truncate">{user}</span>
						<span class="text-xs text-gray-500 truncate">{showActivity(url)}</span>
					</div>
				</div>
			{/each}
		</div>
	</div>
</Menu>

<script lang="ts">
	import { importStore } from '$lib/components/apps/store'

	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { Policy } from '$lib/gen'
	import { page } from '$app/stores'
	import { decodeState } from '$lib/utils'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { userStore } from '$lib/stores'
	import type { App } from '$lib/components/apps/types'
	import { goto } from '$app/navigation'

	let nodraft = $page.url.searchParams.get('nodraft')

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	const importJson = $importStore
	if ($importStore) {
		$importStore = undefined
	}

	const initialState = nodraft ? undefined : localStorage.getItem('app')

	let value: App =
		importJson ??
		(initialState != undefined
			? decodeState(initialState)
			: {
					grid: [],
					title: 'New App',
					fullscreen: false,
					unusedInlineScripts: []
			  })

	$dirtyStore = false
</script>

{#if value}
	<div class="h-screen">
		<AppEditor
			summary={''}
			app={value}
			path={''}
			policy={{
				on_behalf_of: `u/${$userStore?.username}`,
				on_behalf_of_email: $userStore?.email,
				execution_mode: Policy.execution_mode.PUBLISHER
			}}
		/>
	</div>
{/if}

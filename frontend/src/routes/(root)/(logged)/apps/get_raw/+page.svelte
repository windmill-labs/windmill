<script lang="ts">
	import { Skeleton } from '$lib/components/common'
	import { userStore, workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'

	let loaded = false

	onMount(async () => {
		globalThis.windmill = {
			username: $userStore?.username,
			email: $userStore?.email,
			workspace: $workspaceStore
		}

		//@ts-ignore
		await import('http://localhost:3000/app.iife.js')
		loaded = false
	})
</script>

<div id="root" />

{#if !loaded}
	<Skeleton layout={[10]} />
{/if}

<script lang="ts">
	import { page } from '$app/stores'
	import { Skeleton } from '$lib/components/common'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy, onMount } from 'svelte'

	let loaded = false

	onMount(async () => {
		globalThis.windmill = {
			username: $userStore?.username,
			email: $userStore?.email,
			workspace: $workspaceStore
		}

		// //@ts-ignore
		// await import('http://localhost:3000/app.iife.js')
		/* @vite-ignore */
		await import(
			/* webpackIgnore: true */
			`/api/w/${$workspaceStore}/raw_apps/get_data/${$page.params.version}/${$page.params.path}`
		)
		try {
			globalThis.render()
		} catch (e) {
			sendUserToast('App seem to be ill-defined', true)
			console.error(e)
		}
		loaded = true
	})

	onDestroy(() => {
		globalThis.windmill = undefined
	})
</script>

<div id="root"></div>

{#if !loaded}
	<Skeleton layout={[10]} />
{/if}

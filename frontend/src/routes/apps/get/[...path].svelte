<script context="module">
	export function load({ params }) {
		return {
			stuff: { title: `Script ${params.hash}` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Skeleton } from '$lib/components/common'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	let app: AppWithLastVersion | undefined = undefined

	async function loadApp() {
		app = await AppService.getAppByPath({ workspace: $workspaceStore!, path: $page.params.path })
	}
	$: if ($workspaceStore) {
		loadApp()
	}
</script>

<Skeleton loading={app == undefined} layout={[10]} />

<CenteredPage>
	<a href="/apps">Back to apps</a>

	{#if app}
		<a href="/apps/edit/{$page.params.path}">Edit</a>
		<div>{JSON.stringify(app, null, 4)} </div>
	{/if}
</CenteredPage>

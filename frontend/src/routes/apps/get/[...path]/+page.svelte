<script lang="ts">
	import { page } from '$app/stores'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import type { EditorBreakpoint } from '$lib/components/apps/types'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Skeleton } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faPen } from '@fortawesome/free-solid-svg-icons'
	import { writable } from 'svelte/store'

	let app: AppWithLastVersion | undefined = undefined

	async function loadApp() {
		app = await AppService.getAppByPath({ workspace: $workspaceStore!, path: $page.params.path })
	}

	$: if ($workspaceStore) {
		loadApp()
	}

	const breakpoint = writable<EditorBreakpoint>('lg')
</script>

<Skeleton loading={app == undefined} layout={[10]} />

<CenteredPage>
	{#if app}
		<div class="flex justify-between items-center py-4">
			<h2>{app.value.title}</h2>

			<Button size="xs" href="/apps/edit/{$page.params.path}" startIcon={{ icon: faPen }}>
				Edit
			</Button>
		</div>

		<div class="border rounded-md p-2">
			<AppPreview app={app.value} appPath={app.path} {breakpoint} />
		</div>
	{/if}
</CenteredPage>

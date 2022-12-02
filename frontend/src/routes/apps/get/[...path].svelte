<script context="module">
	export function load({ params }) {
		return {
			stuff: { title: `Script ${params.hash}` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Skeleton } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faArrowLeft, faPen } from '@fortawesome/free-solid-svg-icons'

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
	{#if app}
		<div class="flex justify-between my-2 items-center">
			<div>{app.value.title}</div>

			<Button size="xs" href="/apps/edit/{$page.params.path}" startIcon={{ icon: faPen }}>
				Edit
			</Button>
		</div>

		<div class="h-screen">
			<AppEditor app={app.value} path={app.path} initialMode="preview" />
		</div>
	{/if}
</CenteredPage>

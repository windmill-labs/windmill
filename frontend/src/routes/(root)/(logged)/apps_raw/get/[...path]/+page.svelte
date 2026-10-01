<script lang="ts">
	/*
	 * WIN-2006: in-workspace raw app viewer. Thin wrapper over the shared
	 * InWorkspaceAppViewer (same component as the low-code /apps/get route) so raw
	 * apps get the identical sandbox behavior. PublicAppFrame renders raw apps
	 * inline with the bundle isolated in RawAppPreview's own opaque iframe.
	 */
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import InWorkspaceAppViewer from '$lib/components/apps/editor/InWorkspaceAppViewer.svelte'
	import { Skeleton } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'
	import { page } from '$app/state'

	let workspace = $derived($workspaceStore ?? '')
	let path = $derived(page.params.path ?? '')
</script>

<!-- Claimed here rather than in the viewer below: the viewer mounts once the workspace has
     resolved, and until then the band would be an ordinary header on screen, only to leave again. -->
<PageHeaderContent fullBleed />

{#if workspace && path}
	<!-- Key by target: SvelteKit reuses this page component on in-route
	     navigation, so the viewer must fully remount (see /apps/get). -->
	{#key `${workspace}/${path}`}
		<InWorkspaceAppViewer {workspace} {path} />
	{/key}
{:else}
	<Skeleton layout={[10]} />
{/if}

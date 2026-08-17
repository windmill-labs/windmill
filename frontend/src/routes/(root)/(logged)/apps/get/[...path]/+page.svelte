<script lang="ts">
	/*
	 * WIN-2006: in-workspace low-code app viewer. Thin wrapper over the shared
	 * InWorkspaceAppViewer, which renders the app sandboxed (opaque iframe / scoped
	 * token) through the same machinery as the public viewer. Raw apps use the
	 * sibling /apps_raw/get route, which wraps the same component.
	 */
	import { base } from '$lib/base'
	import InWorkspaceAppViewer from '$lib/components/apps/editor/InWorkspaceAppViewer.svelte'
	import { Skeleton } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'
	import { page } from '$app/state'

	let workspace = $derived($workspaceStore ?? '')
	let path = $derived(page.params.path ?? '')
</script>

<!-- Wait for the active workspace before mounting the embedder: it's needed to
     mint the token and to build the viewer iframe URL, and the store is set
     asynchronously by the (logged) layout. -->
{#if workspace && path}
	<!-- Key by target: SvelteKit reuses this page component on in-route
	     navigation (e.g. a navbar item linking to another app), so the viewer
	     must fully remount — otherwise the previous app (and in sandbox mode its
	     path-scoped token) sticks around. -->
	{#key `${workspace}/${path}`}
		<InWorkspaceAppViewer {workspace} {path} editHref="{base}/apps/edit/{path}?nodraft=true" />
	{/key}
{:else}
	<Skeleton layout={[10]} />
{/if}

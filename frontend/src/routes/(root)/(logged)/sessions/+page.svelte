<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	// The sessions view is the logged layout's, so its chats outlive this route. Imported here
	// only so that preloading and navigating to the route fetch its code, as for any page.
	import '$lib/components/sessions/SessionsView.svelte'

	// The sessions UI hosts preview iframes that load Windmill pages. One navigating back here
	// would nest another sessions UI, with its own iframes, endlessly; the layout does not
	// mount it in a frame.
	const embedded = typeof window !== 'undefined' && window.self !== window.top
</script>

{#if embedded}
	<div class="p-8 flex flex-col items-start gap-3 text-secondary text-sm">
		<p class="text-primary font-medium">Sessions can't open inside a preview</p>
		<p>This page is being previewed in a session panel. Open it at the top level instead.</p>
		<Button
			unifiedSize="sm"
			startIcon={{ icon: ExternalLink }}
			onclick={() => {
				const u = new URL(window.location.href)
				u.searchParams.delete('nomenubar')
				window.top?.location.assign(u.pathname + u.search)
			}}>Open sessions</Button
		>
	</div>
{/if}

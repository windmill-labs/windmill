<!--
@component
The answer row of a change held outside the session's scope, laid out like a tool confirmation's
footer and placed where one goes. YOLO never answers it, and Stop leaves it unanswered.
-->
<script lang="ts">
	import { Button } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	import { getChatViewHost } from './chatViewHost'
	import type { ScopeDecision } from './workingScope'

	let {
		toolCallId,
		path,
		root,
		scope,
		verb,
		class: className
	}: {
		toolCallId: string
		path: string
		root: string
		scope: string
		verb: string
		class?: string
	} = $props()

	const chatHost = getChatViewHost()

	function answer(decision: ScopeDecision) {
		chatHost.handleScopeDecision(toolCallId, decision)
	}
</script>

<!-- font-main: the tool cards around it are font-mono, which reads as bold on a button label. -->
<div class={twMerge('flex flex-col gap-2 font-main', className)}>
	<p class="text-xs text-primary">
		This would {verb} <span class="font-mono">{path}</span>, outside
		<span class="font-mono">{scope}</span>.
	</p>
	<div class="flex flex-row flex-wrap items-center justify-end gap-2">
		<Button variant="default" unifiedSize="sm" on:click={() => answer('add')}>
			Allow writes in <span class="font-mono">{root}</span>
		</Button>
		<Button variant="accent" unifiedSize="sm" on:click={() => answer('once')}>Allow once</Button>
	</div>
</div>

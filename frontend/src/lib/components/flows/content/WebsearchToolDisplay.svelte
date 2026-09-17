<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'

	let {
		noEditor = false,
		agentTrail = undefined
	}: {
		noEditor?: boolean
		/** See `FlowCardHeader`. */
		agentTrail?: Pick<FlowModule, 'id' | 'summary'>[]
	} = $props()
</script>

<FlowCard {noEditor} {agentTrail} title="Web search">
	<div class="flex flex-col gap-4 overflow-auto p-4" style="scrollbar-gutter: stable">
		<Alert title="Web search tool" type="info">
			Gives the AI Agent the ability to search the web. Only works for openai, anthropic and google
			models for now.
		</Alert>
	</div>
</FlowCard>

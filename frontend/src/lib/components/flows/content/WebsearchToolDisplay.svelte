<script lang="ts">
	import FlowCard from '../common/FlowCard.svelte'

	let { noEditor = false }: { noEditor?: boolean } = $props()
</script>

<FlowCard {noEditor} title="Web search">
	<div class="flex flex-col gap-4 overflow-auto p-4" style="scrollbar-gutter: stable">
		<p class="text-xs font-normal text-secondary">
			Lets the agent search the web. Works with OpenAI, Anthropic and Google models.
		</p>
	</div>
</FlowCard>

<script lang="ts">
	import FlowCard from '../common/FlowCard.svelte'

	let { noEditor = false }: { noEditor?: boolean } = $props()
</script>

<FlowCard
	{noEditor}
	title="Web search"
	subtitle="Works with OpenAI, Anthropic and Google models."
	subtitleDocLink="https://www.windmill.dev/docs/core_concepts/ai_agents"
/>

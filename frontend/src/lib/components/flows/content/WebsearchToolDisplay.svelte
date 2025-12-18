<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'

	interface Props {
		noEditor?: boolean
	}

	let { noEditor = false }: Props = $props()
</script>

<div class="flex flex-col gap-4 p-4">
	<Alert title="Web search tool" type="info">
		Gives the AI Agent the ability to search the web. Only works for openai, anthropic and google
		models for now.
	</Alert>
</div>

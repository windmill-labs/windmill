<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import type { DisplayMessage } from './shared'
	import CodeDisplay from './script/CodeDisplay.svelte'
	import { setContext } from 'svelte'

	export let message: DisplayMessage

	setContext('AssistantMessageContext', {
		message
	})
</script>

<div
	class="prose prose-sm dark:prose-invert w-full max-w-full leading-snug space-y-2 prose-ul:!pl-6"
>
	<Markdown
		md={message.content}
		plugins={[
			gfmPlugin(),
			{
				renderer: {
					pre: CodeDisplay
				}
			}
		]}
	/>
</div>

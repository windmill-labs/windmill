<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import type { FlowDisplayMessage } from './core'
	import { setContext } from 'svelte'
	import CodeDisplay from '../chat/CodeDisplay.svelte'

	export let message: FlowDisplayMessage

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

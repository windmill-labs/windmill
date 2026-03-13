<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import type { DisplayMessage } from './shared'
	import CodeDisplay from './script/CodeDisplay.svelte'
	import LinkRenderer from './LinkRenderer.svelte'

	interface Props {
		message: DisplayMessage;
	}

	let { message }: Props = $props();
</script>

<div
	class="prose prose-xs dark:prose-invert w-full max-w-full leading-snug space-y-2 prose-ul:!pl-6 !text-2xs"
>
	<Markdown
		md={message.content}
		plugins={[
			gfmPlugin(),
			{
				renderer: {
					pre: CodeDisplay,
					a: LinkRenderer
				}
			}
		]}
	/>
</div>

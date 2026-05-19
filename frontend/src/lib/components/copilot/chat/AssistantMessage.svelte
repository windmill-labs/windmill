<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import type { DisplayMessage } from './shared'
	import CodeDisplay from './script/CodeDisplay.svelte'
	import LinkRenderer from './LinkRenderer.svelte'

	interface Props {
		message: DisplayMessage
	}

	let { message }: Props = $props()
</script>

<div
	class="prose prose-sm dark:prose-invert w-full max-w-full leading-snug space-y-2 prose-ul:!pl-6
		prose-p:text-xs prose-li:text-xs prose-code:text-xs prose-pre:text-xs
		prose-headings:font-medium prose-headings:text-emphasis prose-headings:mt-3 prose-headings:mb-1
		prose-h1:text-sm prose-h2:text-xs prose-h3:text-xs prose-h4:text-xs prose-h5:text-xs prose-h6:text-xs"
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

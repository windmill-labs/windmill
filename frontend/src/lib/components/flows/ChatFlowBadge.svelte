<script lang="ts">
	import { MessageSquare } from 'lucide-svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	interface Props {
		title?: string
	}

	let { title = 'Chat input enabled: this flow opens as a conversation' }: Props = $props()
</script>

<Badge
	small
	color="gray"
	class="px-1.5 shrink-0"
	icon={{ icon: MessageSquare, position: 'left' }}
	{title}
>
	Chat
</Badge>

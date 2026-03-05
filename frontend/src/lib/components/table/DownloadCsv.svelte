<script lang="ts">
	import { Download } from 'lucide-svelte'
	import { Button } from '../common'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		getContent: () => string;
		customText?: string | undefined;
	}

	let { getContent, customText = undefined }: Props = $props();
</script>

<Button
	unifiedSize="md"
	variant="subtle"
	startIcon={{ icon: Download }}
	on:click={() => {
		try {
			const blob = new Blob([getContent()], { type: 'text/csv;charset=utf-8;' })
			const url = URL.createObjectURL(blob)
			const link = document.createElement('a')
			link.setAttribute('href', url)
			link.setAttribute('download', 'data.csv')
			link.style.visibility = 'hidden'
			document.body.appendChild(link)
			link.click()

			document.body.removeChild(link)
		} catch (err) {
			sendUserToast(err, true)
		}
	}}
>
	{customText || 'Download as CSV'}
</Button>

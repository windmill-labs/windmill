<script lang="ts">
	import { Download } from 'lucide-svelte'
	import { Button } from '../common'
	import { sendUserToast } from '$lib/toast'

	export let getContent: () => string
	export let customText: string | undefined = undefined
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

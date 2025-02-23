<script>
	import { Button } from '$lib/components/common'
</script>

<div class="flex flex-col bg-black rounded-lg p-4 relative">
	<Button
		class="bg-green-500 px-1 py-0.5 absolute top-0 right-0 rounded-tr-lg rounded-bl-lg text-xs"
	>
		Preview
	</Button>
	<slot />
</div>

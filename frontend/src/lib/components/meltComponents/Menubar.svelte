<script lang="ts">
	import { createMenubar, melt } from '@melt-ui/svelte'

	const {
		elements: { menubar },
		builders: { createMenu }
	} = createMenubar()

	let className: string = ''
	export { className as class }
</script>

<div use:melt={$menubar} class={className}>
	<slot {createMenu} />
</div>

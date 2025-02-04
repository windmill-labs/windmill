<script lang="ts">
	import { createMenubar, melt } from '@melt-ui/svelte'

	const {
		elements: { menubar },
		builders: { createMenu }
	} = createMenubar()
</script>

<div use:melt={$menubar}>
	<slot {createMenu} />
</div>

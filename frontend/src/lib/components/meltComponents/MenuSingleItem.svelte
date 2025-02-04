<script lang="ts">
	import { melt } from '@melt-ui/svelte'

	export let createMenu: () => any

	const menu = createMenu()

	const {
		elements: { trigger, item, menu: menuElement }
	} = menu
</script>

<div class="hover:z-50 flex w-full h-8">
	<slot name="trigger" trigger={$trigger} />
	<div use:melt={$menuElement}>
		<slot item={$item} />
	</div>
</div>

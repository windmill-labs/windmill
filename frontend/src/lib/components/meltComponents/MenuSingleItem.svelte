<script lang="ts">
	import { melt, type MenubarBuilders } from '@melt-ui/svelte'

	export let createMenu: MenubarBuilders['createMenu']

	const menu = createMenu()

	const {
		elements: { trigger, item, menu: menuElement }
	} = menu
</script>

<div class="hover:z-50 flex w-full h-8">
	<slot name="trigger" {trigger} />
	<div use:melt={$menuElement}>
		<slot {item} />
	</div>
</div>

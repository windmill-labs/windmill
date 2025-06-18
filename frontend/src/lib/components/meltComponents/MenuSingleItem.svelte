<script lang="ts">
	import { melt, type MenubarBuilders } from '@melt-ui/svelte'

	interface Props {
		createMenu: MenubarBuilders['createMenu']
		triggr?: import('svelte').Snippet<[any]>
		children?: import('svelte').Snippet<[any]>
	}

	let { createMenu, triggr, children }: Props = $props()

	const menu = createMenu()

	const {
		elements: { trigger, item, menu: menuElement }
	} = menu
</script>

<div class="hover:z-50 flex w-full h-8">
	{@render triggr?.({ trigger })}
	<div use:melt={$menuElement}>
		{@render children?.({ item })}
	</div>
</div>

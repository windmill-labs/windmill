<script lang="ts">
	import { createMenubar, melt } from '@melt-ui/svelte'
	interface Props {
		children?: import('svelte').Snippet<[any]>;
		[key: string]: any
	}

	let { ...props }: Props = $props();

	const {
		elements: { menubar },
		builders: { createMenu }
	} = createMenubar()
</script>

<div use:melt={$menubar} class={props.class}>
	{@render props.children?.({ createMenu, })}
</div>

<script lang="ts">
	import { createMenubar, melt } from '@melt-ui/svelte'

	const {
		elements: { menubar },
		builders: { createMenu }
	} = createMenubar()

	interface Props {
		class?: string;
		children?: import('svelte').Snippet<[any]>;
	}

	let { class: className = '', children }: Props = $props();
	
</script>

<div use:melt={$menubar} class={className}>
	{@render children?.({ createMenu, })}
</div>

<script lang="ts">
	import { melt, type AnyMeltElement } from '@melt-ui/svelte'

	export let meltElement: AnyMeltElement
	export let type: 'button' | 'submit' | 'reset' | null | undefined = 'button'
	export let title: string = ''
	export let id: string = ''
</script>

<button use:melt={$meltElement} class={$$props.class} {type} {title} {id}>
	<slot />
</button>

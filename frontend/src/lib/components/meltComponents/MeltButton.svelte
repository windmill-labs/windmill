<script lang="ts">
	import { type AnyMeltElement } from '@melt-ui/svelte'

	export let meltElement: AnyMeltElement
	export let type: 'button' | 'submit' | 'reset' | null | undefined = undefined
	export let title: string = ''
	export let id: string | undefined = undefined
</script>

<button use:meltElement class={$$props.class} {type} {title} {id} {...$meltElement}>
	<slot />
</button>

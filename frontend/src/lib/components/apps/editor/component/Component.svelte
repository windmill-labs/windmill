<script lang="ts">
	import ComponentInner from './ComponentInner.svelte'
	import ComponentRendered from './ComponentRendered.svelte'
	import type { AppComponent } from './components'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let render: boolean
	export let hidden: boolean
	export let fullHeight: boolean
</script>

{#if render}
	<ComponentRendered {component} {selected} {locked} {render} {hidden} {fullHeight} />
{:else}
	<ComponentInner
		{component}
		{render}
		componentContainerHeight={0}
		errorHandledByComponent={false}
		inlineEditorOpened={false}
	/>
{/if}

<script lang="ts">
	import { ArrowLeft, ArrowRight } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Select from '../apps/svelte-select/lib/Select.svelte'
	import Button from '../common/button/Button.svelte'
	import Popup from '../common/popup/Popup.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'

	export let items: string[] = []

	const dispatch = createEventDispatcher()
	let value = ''
	let darkMode = false
</script>

<DarkModeObserver bind:darkMode />

<Popup
	let:close
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	shouldUsePortal={false}
>
	<svelte:fragment slot="button">
		<Button
			startIcon={{
				icon: ArrowLeft
			}}
			size="xs"
			color="light"
			nonCaptureEvent>Pick another app as target href</Button
		>
	</svelte:fragment>

	<div class="w-80 flex flex-col gap-2">
		<Select
			class="grow shrink max-w-full"
			on:change
			bind:value
			{items}
			placeholder="Pick an app"
			inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
			containerStyles={darkMode
				? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
				: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
			portal={false}
		/>

		<Button
			size="xs"
			color="light"
			on:click={() => {
				dispatch('pick', value)
				close(null)
				value = ''
			}}
			disabled={!value}
		>
			<ArrowRight size={18} />
		</Button>
	</div>
</Popup>

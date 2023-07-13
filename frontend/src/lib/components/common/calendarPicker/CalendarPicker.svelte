<script lang="ts">
	import { ArrowRight, Calendar } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { Button, Popup } from '..'

	export let date: string | undefined
	export let label: string

	const dispatch = createEventDispatcher()
	let value = date
	let button: HTMLButtonElement
	let input: HTMLInputElement

	function save() {
		dispatch('change', value)
		input.blur()
	}

	function close() {
		const elem = document.activeElement as HTMLElement
		if (elem.blur) {
			elem.blur()
		}
	}
</script>

<button
	bind:this={button}
	title="Open calendar picker"
	class="absolute bottom-1 right-1 top-1 py-1 min-w-min !px-2 items-center text-gray-800 bg-white border rounded center-center hover:bg-gray-50 transition-all cursor-pointer"
	aria-label="Open calendar picker"
>
	<Calendar size={14} />
</button>
<Popup
	ref={button}
	options={{ placement: 'top-start' }}
	transition={fade}
	wrapperClasses="!z-[1002]"
	outerClasses="rounded shadow-xl bg-white border p-3"
	closeOn={[]}
>
	<label class="block text-gray-900">
		<div class="pb-1 text-sm text-gray-600">{label}</div>
		<div class="flex w-full">
			<input type="datetime-local" bind:value class="!w-auto grow" bind:this={input} />
			<Button
				size="xs"
				color="dark"
				buttonType="button"
				btnClasses="!p-1 !w-[34px] !ml-1"
				aria-label="Save ID"
				on:click={() => {
					save()
					close()
				}}
			>
				<ArrowRight size={18} />
			</Button>
		</div>
	</label>
</Popup>

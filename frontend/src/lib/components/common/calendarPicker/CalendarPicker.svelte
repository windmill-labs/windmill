<script lang="ts">
	import { ArrowRight, Calendar } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { Button, Popup } from '..'

	export let date: string | undefined
	export let label: string

	const dispatch = createEventDispatcher()
	let value = date
	let button: HTMLButtonElement
	let input: HTMLInputElement

	$: if (date && input) {
		input.value = new Date(date).toISOString().slice(0, 16)
	}

	function save() {
		dispatch('change', value)
		input.blur()
	}
</script>

<Popup floatingConfig={{ placement: 'top-start', strategy: 'absolute' }} let:close>
	<svelte:fragment slot="button">
		<button
			bind:this={button}
			title="Open calendar picker"
			class="absolute bottom-1 right-1 top-1 py-1 min-w-min !px-2 items-center text-gray-800 bg-white border rounded center-center hover:bg-gray-50 transition-all cursor-pointer"
			aria-label="Open calendar picker"
			on:click={() => {
				input.focus()
			}}
		>
			<Calendar size={14} />
		</button>
	</svelte:fragment>

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
					close(null)
				}}
			>
				<ArrowRight size={18} />
			</Button>
		</div>
	</label>
</Popup>

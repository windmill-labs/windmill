<script lang="ts">
	import { ArrowRight, Pencil } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { fade, slide } from 'svelte/transition'
	import { Button, Popup } from '../../../../common'

	export let id: string
	const dispatch = createEventDispatcher()
	const regex = /^[a-zA-Z0-9]{1,}$/
	let value = id
	let button: HTMLButtonElement
	let input: HTMLInputElement
	let error = false

	$: if (regex.test(value)) {
		id = value
		error = false
	} else {
		error = true
	}

	function save() {
		if (error) {
			return
		}
		dispatch('change', id)
		input.blur()
	}
</script>

<button
	on:click|stopPropagation={() => {
		value = id
	}}
	bind:this={button}
	class="rounded-sm bg-gray-100 hover:text-black text-gray-600 px-1"
	aria-label="Open component ID editor"
>
	<Pencil size={14} />
</button>
<Popup
	ref={button}
	options={{ placement: 'top-start' }}
	transition={fade}
	wrapperClasses="!z-[1002]"
	outerClasses="rounded shadow-xl bg-white border p-3"
>
	<label class="block w-[220px] text-gray-900">
		<div class="pb-1 text-sm text-gray-600">Component ID</div>
		<div class="flex w-full">
			<input
				type="text"
				bind:value
				class="!w-auto grow"
				bind:this={input}
				on:click|stopPropagation={() => {}}
				on:keypress={({ key }) => {
					if (key === 'Enter') save()
				}}
			/>
			<Button
				size="xs"
				color="blue"
				buttonType="button"
				btnClasses="!p-1 !w-[34px] !ml-1"
				aria-label="Save ID"
				on:click={save}
			>
				<ArrowRight size={18} />
			</Button>
		</div>
		{#if error}
			<div
				transition:slide|local={{ duration: 200 }}
				class="w-full text-sm text-red-600 whitespace-pre-wrap pt-1"
			>
				The ID must include only letters and numbers
			</div>
		{/if}
	</label>
</Popup>

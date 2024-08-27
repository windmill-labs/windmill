<script lang="ts">
	import { ArrowRight, Pencil } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from './common/button/Button.svelte'
	import Popup from './common/popup/Popup.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'

	export let value: string | undefined
	const dispatch = createEventDispatcher()

	function save() {
		dispatch('save', value)
	}
</script>

<Popup
	let:close
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
>
	<svelte:fragment slot="button">
		<Button nonCaptureEvent={true} size="xs" color="light" startIcon={{ icon: Pencil }}>
			Edit name
		</Button>
	</svelte:fragment>
	<label class="block text-primary">
		<div class="pb-1 text-sm text-secondary">Name</div>
		<div class="flex w-full">
			<input
				type="text"
				bind:value
				class="!w-auto grow"
				on:click|stopPropagation={() => {}}
				on:keydown|stopPropagation
				on:keypress|stopPropagation={({ key }) => {
					if (key === 'Enter') {
						save()
						close(null)
					}
				}}
			/>
			<Button
				size="xs"
				color="blue"
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

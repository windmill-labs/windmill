<script lang="ts">
	import { Settings } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from './common/button/Button.svelte'
	import Popup from './common/popup/Popup.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import ChangeInstanceUsernameInner from './ChangeInstanceUsernameInner.svelte'

	export let value: string | undefined
	export let email: string
	export let username: string | undefined = undefined
	export let automateUsernameCreation: boolean = false

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
		<Button nonCaptureEvent={true} size="xs" color="light" startIcon={{ icon: Settings }} />
	</svelte:fragment>
	<div class="flex flex-col gap-8 max-w-sm">
		{#if automateUsernameCreation && username}
			<ChangeInstanceUsernameInner {email} {username} on:renamed />
		{/if}
		<label class="block text-primary">
			<div class="pb-1 text-xs text-secondary">Name</div>
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
			</div>
			<Button
				size="xs"
				color="blue"
				buttonType="button"
				btnClasses="mt-2 "
				aria-label="Save ID"
				on:click={() => {
					save()
					close(null)
				}}
			>
				Update name
			</Button>
		</label>
	</div>
</Popup>

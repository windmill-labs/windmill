<script lang="ts">
	import { Button, Popup } from './common'
	import { autoPlacement } from '@floating-ui/core'
	import ChangeInstanceUsernameInner from './ChangeInstanceUsernameInner.svelte'

	export let email: string
	export let username: string
	export let isConflict = false
</script>

<Popup
	floatingConfig={{
		middleware: [
			autoPlacement({
				allowedPlacements: ['bottom-end', 'top-end']
			})
		]
	}}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
	let:close
>
	<svelte:fragment slot="button">
		<Button color={isConflict ? 'red' : 'light'} size="xs" spacingSize="xs2" nonCaptureEvent={true}
			>{isConflict ? 'Fix username conflict' : 'Change username'}</Button
		>
	</svelte:fragment>
	<ChangeInstanceUsernameInner
		{email}
		{username}
		{isConflict}
		on:close={() => close(null)}
		on:renamed
	/>
</Popup>

<script lang="ts">
	import { Button } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import RolePicker from './settings/RolePicker.svelte'
	import type { EffectiveRole } from '$lib/gen'
	import { DEVELOPER_ROLE } from '$lib/workspaceRoles'
	import { MailPlus } from 'lucide-svelte'

	interface Props {
		/** `selected` is a role name, or `admin`. */
		inviteUser: (email: string, selected: string) => Promise<void>
		/** The workspace's roles; undefined while loading. */
		roles: EffectiveRole[] | undefined
	}

	const { inviteUser, roles }: Props = $props()

	let email: string = $state('')

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			inviteUser(email, selected)
			email = ''
		}
	}

	let selected: string = $state(DEVELOPER_ROLE)
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	{#snippet trigger()}
		<Button
			variant="default"
			unifiedSize="md"
			nonCaptureEvent={true}
			startIcon={{ icon: MailPlus }}
		>
			Invite
		</Button>
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col gap-2 p-4">
			<input
				type="email"
				onkeyup={handleKeyUp}
				placeholder="email"
				bind:value={email}
				class="mr-4"
			/>
			<RolePicker {roles} disablePortal value={selected} onchange={(v) => (selected = v)} />
			<Button
				variant="accent"
				unifiedSize="md"
				on:click={() => inviteUser(email, selected)}
				disabled={!email || email.trim() === ''}
			>
				Invite
			</Button>
		</div>
	{/snippet}
</Popover>

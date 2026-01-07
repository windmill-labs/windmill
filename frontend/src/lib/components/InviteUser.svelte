<script lang="ts">
	import { Button } from './common'
	import Popover from './meltComponents/Popover.svelte'

	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { MailPlus } from 'lucide-svelte'

	interface Props {
		inviteUser: (email: string, selected: 'operator' | 'developer' | 'admin') => Promise<void>
	}

	const { inviteUser }: Props = $props()

	let email: string = $state('')

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			inviteUser(email, selected)
			email = ''
		}
	}

	let selected: 'operator' | 'developer' | 'admin' = $state('developer')
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
			<ToggleButtonGroup bind:selected>
				{#snippet children({ item })}
					<ToggleButton
						value="operator"
						label="Operator"
						tooltip="An operator can only execute and view scripts/flows/apps from your workspace, and only those that he has visibility on."
						{item}
					/>

					<ToggleButton
						value="developer"
						label="Developer"
						tooltip="A Developer can execute and view scripts/flows/apps, but they can also create new ones and edit those they are allowed to by their path (either u/ or Writer or Admin of their folder found at /f)."
						{item}
					/>

					<ToggleButton
						value="admin"
						label="Admin"
						tooltip="An admin has full control over a specific Windmill workspace, including the ability to manage users, edit entities, and control permissions within the workspace."
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
			<Button
				variant="accent"
				unifiedSize="md"
				on:click={() => inviteUser(email, selected)}
				disabled={email === undefined}
			>
				Invite
			</Button>
		</div>
	{/snippet}
</Popover>

<script lang="ts">
	import { GroupService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Label from '$lib/components/Label.svelte'
	import CollapseLink from '$lib/components/CollapseLink.svelte'
	import { Plus } from 'lucide-svelte'

	interface Props {
		onCreated: (name: string) => void
		variant?: 'accent' | 'default'
		unifiedSize?: 'sm' | 'md'
	}

	let { onCreated, variant = 'accent', unifiedSize = 'md' }: Props = $props()

	let name = $state('')
	let externalId = $state('')

	async function create(close: () => void) {
		const created = name
		await GroupService.createInstanceGroup({
			requestBody: { name: created, external_id: externalId }
		})
		name = ''
		externalId = ''
		close()
		onCreated(created)
	}
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	{#snippet trigger()}
		<Button {variant} {unifiedSize} startIcon={{ icon: Plus }} nonCaptureEvent>New group</Button>
	{/snippet}
	{#snippet content({ close })}
		<div class="flex flex-col w-80 p-4 gap-4">
			<span class="text-sm font-semibold text-emphasis">New instance group</span>
			<Label label="Name">
				<TextInput bind:value={name} size="sm" inputProps={{ placeholder: 'operators' }} />
			</Label>
			<CollapseLink text="Link to an IdP group">
				<Label label="IdP group">
					<span class="text-xs text-secondary">
						SSO logins then set the members from the IdP. SCIM links the groups it creates on its
						own, so link here only a group SCIM doesn't manage.
					</span>
					<TextInput bind:value={externalId} size="sm" inputProps={{ placeholder: '/operators' }} />
				</Label>
			</CollapseLink>
			<Button variant="accent" unifiedSize="sm" disabled={!name} onclick={() => create(close)}>
				Create
			</Button>
		</div>
	{/snippet}
</Popover>

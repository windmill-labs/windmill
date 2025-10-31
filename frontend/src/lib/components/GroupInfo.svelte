<script lang="ts">
	import { GroupService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Popover from './Popover.svelte'
	import { untrack } from 'svelte'

	interface Props {
		name: string
	}

	let { name }: Props = $props()

	let members: string[] | undefined = $state([])

	async function loadMembers() {
		// FAKE DATA FOR TESTING - Set to false to remove all fake data
		const ENABLE_FAKE_DATA = false

		if (ENABLE_FAKE_DATA) {
			// Fake member data for different groups
			const fakeMembers: Record<string, string[]> = {
				developers: ['john.doe', 'jane.smith', 'bob.wilson', 'alice.johnson', 'charlie.brown'],
				'readonly-users': ['viewer1', 'viewer2'],
				admins: ['admin1', 'admin2', 'admin3'],
				testers: ['tester1', 'tester2', 'tester3', 'tester4'],
				'external-contractors': ['contractor1', 'contractor2']
			}

			if (fakeMembers[name]) {
				members = fakeMembers[name]
				return
			}
		}

		try {
			members = (await GroupService.getGroup({ workspace: $workspaceStore!, name })).members
		} catch (e) {
			members = []
		}
	}
	$effect(() => {
		$workspaceStore && untrack(() => loadMembers())
	})
</script>

{#if members}
	<Popover
		><div class="inline-flex gap-1 items-end text-primary text-xs font-normal"
			><span>({members.length})</span>
			<div class="max-w-xs truncate"><span>{members?.join(', ')}</span></div></div
		>
		{#snippet text()}
			<span>{members?.join(', ')}</span>
		{/snippet}</Popover
	>
{/if}

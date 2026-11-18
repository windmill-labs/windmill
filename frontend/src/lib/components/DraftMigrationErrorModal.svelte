<script lang="ts">
	/**
	 * Lists drafts that `migrateUserDraftsToDb` failed to push to the server.
	 * Opened from the single "Resolve issues" toast (or stays empty/closed when
	 * nothing failed). Reads the registry live, so failures that surface while
	 * the modal is already open appear in place. Mount once, app-wide.
	 */
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Braces, Trash2 } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import {
		draftMigrationErrors,
		deleteDraftMigrationError,
		deleteAllDraftMigrationErrors,
		type DraftMigrationError
	} from '$lib/userDraftMigrationErrors.svelte'

	const DRAFT_DOCS_URL = 'https://www.windmill.dev/docs/core_concepts/draft_and_deploy'

	let jsonView = $state<DraftMigrationError | undefined>(undefined)
	let jsonOpen = $state(false)

	function viewJson(error: DraftMigrationError) {
		jsonView = error
		jsonOpen = true
	}
</script>

<Modal2
	bind:isOpen={draftMigrationErrors.modalOpen}
	title="Resolve draft migration issues"
	fixedWidth="md"
	fixedHeight="adaptive"
	closeOnOutsideClick={!jsonOpen}
>
	<div class="flex flex-col w-full gap-4">
		<p class="text-sm text-secondary">
			Drafts are now user-scoped and synced to the database.
			{#if draftMigrationErrors.list.length}
				These local storage drafts could not be
				migrated to the server — view their contents before deciding, then delete the ones you no
				longer need.
			{/if}
			<a href={DRAFT_DOCS_URL} target="_blank" rel="noopener noreferrer">Learn more</a>.
		</p>

		{#if draftMigrationErrors.list.length === 0}
			<p class="text-sm text-tertiary italic">All issues resolved.</p>
		{:else}
			<div class="flex justify-end">
				<Button
					color="red"
					variant="default"
					size="xs"
					startIcon={{ icon: Trash2 }}
					on:click={() => deleteAllDraftMigrationErrors()}
				>
					Remove all
				</Button>
			</div>
			<ul class="divide-y border-t border-b flex-1 overflow-y-auto max-h-72">
				{#each draftMigrationErrors.list as error (error.key)}
					<li class="flex items-center gap-3 py-2">
						<div class="flex-1 min-w-0">
							<div class="text-sm font-medium text-primary truncate">{error.path}</div>
							<div class="text-xs text-tertiary truncate">
								{error.itemKind} · {error.workspace}
							</div>
						</div>
						<Button
							variant="default"
							size="xs"
							startIcon={{ icon: Braces }}
							on:click={() => viewJson(error)}
						>
							View JSON
						</Button>
						<Button
							color="red"
							variant="default"
							size="xs"
							startIcon={{ icon: Trash2 }}
							on:click={() => deleteDraftMigrationError(error.key)}
						>
							Delete draft
						</Button>
					</li>
				{/each}
			</ul>
		{/if}

		<div class="flex justify-end">
			<Button variant="default" size="sm" on:click={() => (draftMigrationErrors.modalOpen = false)}>
				Close
			</Button>
		</div>
	</div>
</Modal2>

<Modal2
	bind:isOpen={jsonOpen}
	title="Draft JSON — {jsonView?.path ?? ''}"
	fixedWidth="lg"
	fixedHeight="lg"
>
	{#snippet headerRight()}
		<Button
			variant="default"
			size="xs"
			on:click={() => {
				navigator.clipboard?.writeText(JSON.stringify(jsonView?.value ?? {}, null, 2))
				sendUserToast('Copied to clipboard')
			}}
		>
			Copy
		</Button>
	{/snippet}
	<div class="w-full overflow-auto">
		<pre class="text-xs whitespace-pre font-mono bg-surface-secondary rounded p-3"
			>{JSON.stringify(jsonView?.value ?? {}, null, 2)}</pre
		>
	</div>
</Modal2>

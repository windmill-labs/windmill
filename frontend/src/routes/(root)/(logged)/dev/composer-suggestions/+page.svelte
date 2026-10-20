<script lang="ts">
	// DEV-ONLY playground for the composer's suggestion rules: pick a role, edit the signals by
	// hand or load a workspace's real ones, and see the pills the starter would show. Not
	// linked anywhere; open at /dev/composer-suggestions. Only reachable on dev builds.
	import { Badge, Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import ComposerSuggestions from '$lib/components/copilot/chat/starter/ComposerSuggestions.svelte'
	import {
		CHANNEL_TYPES,
		composerSuggestions,
		DATABASE_TYPES,
		type ComposerRole,
		type ComposerSignals
	} from '$lib/components/copilot/chat/starter/composerSuggestions'
	import { readComposerSignals } from '$lib/components/copilot/chat/starter/starterPrompts.svelte'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { Download } from 'lucide-svelte'

	type RoleName = 'operator' | 'developer' | 'workspaceAdmin' | 'instanceAdmin'
	const ROLES: Record<RoleName, ComposerRole> = {
		operator: { operator: true, workspaceAdmin: false, instanceAdmin: false },
		developer: { operator: false, workspaceAdmin: false, instanceAdmin: false },
		workspaceAdmin: { operator: false, workspaceAdmin: true, instanceAdmin: false },
		instanceAdmin: { operator: false, workspaceAdmin: true, instanceAdmin: true }
	}

	// Every field as the form holds it: a blank field is an absent signal, which fires no rule.
	type Form = {
		itemCount: string
		draftPath: string
		deployPath: string
		sharedFlowPath: string
		favoritePath: string
		databasePath: string
		databaseType: string
		channelPath: string
		channelType: string
	}
	const BLANK: Form = {
		itemCount: '',
		draftPath: '',
		deployPath: '',
		sharedFlowPath: '',
		favoritePath: '',
		databasePath: '',
		databaseType: 'postgresql',
		channelPath: '',
		channelType: 'slack'
	}
	const PRESETS: Record<string, Partial<Form>> = {
		'Empty workspace': { itemCount: '0' },
		'Empty, connected': {
			itemCount: '0',
			databasePath: 'f/data/main_db',
			channelPath: 'f/team/slack'
		},
		'Has items': { itemCount: '25' },
		'Own work': {
			itemCount: '25',
			draftPath: 'f/billing/sync_invoices',
			deployPath: 'u/admin/etl_nightly'
		},
		Established: {
			itemCount: '25',
			sharedFlowPath: 'f/billing/charge_customers',
			favoritePath: 'f/ops/check_ssl_expiry',
			databasePath: 'f/data/main_db'
		}
	}
	type PathKey = 'draftPath' | 'deployPath' | 'sharedFlowPath' | 'favoritePath'
	const PATH_FIELDS: { key: PathKey; label: string }[] = [
		{ key: 'draftPath', label: 'My latest draft (flow path)' },
		{ key: 'deployPath', label: 'Latest failed deploy (script path)' },
		{ key: 'sharedFlowPath', label: 'Newest shared flow (path)' },
		{ key: 'favoritePath', label: 'Favorite (script path)' }
	]
	const DATABASE_ITEMS = Object.entries(DATABASE_TYPES).map(([value, label]) => ({ value, label }))
	const CHANNEL_ITEMS = Object.keys(CHANNEL_TYPES).map((value) => ({ value, label: value }))

	let roleName = $state<RoleName>('instanceAdmin')
	let form = $state<Form>({ ...BLANK, ...PRESETS['Has items'] })
	let workspace = $state<string | undefined>($workspaceStore ?? undefined)
	let loading = $state(false)
	let picked = $state<string>()

	const num = (s: string) => (s.trim() === '' || isNaN(Number(s)) ? undefined : Number(s))

	const item = (kind: string, path: string) =>
		path.trim() ? { kind, path: path.trim(), name: path.trim().split('/').pop()! } : undefined
	const connection = (type: string, path: string) => {
		const i = item(type, path)
		return i && { type: i.kind, path: i.path, name: i.name }
	}

	const signals: ComposerSignals = $derived({
		itemCount: num(form.itemCount),
		myDraft: item('flow', form.draftPath),
		failedDeploy: item('script', form.deployPath),
		sharedFlow: item('flow', form.sharedFlowPath),
		favorite: item('script', form.favoritePath),
		database: connection(form.databaseType, form.databasePath),
		channel: connection(form.channelType, form.channelPath)
	})
	const suggestions = $derived(composerSuggestions(signals, ROLES[roleName]))

	function applyPreset(name: string) {
		form = { ...BLANK, ...PRESETS[name] }
		picked = undefined
	}

	async function loadWorkspace() {
		if (!workspace) return
		loading = true
		// A failed list reads as 10 items with nothing of yours; the console says why.
		const s = await readComposerSignals(workspace)
		form = {
			itemCount: String(s.itemCount ?? ''),
			draftPath: s.myDraft?.path ?? '',
			deployPath: s.failedDeploy?.path ?? '',
			sharedFlowPath: s.sharedFlow?.path ?? '',
			favoritePath: s.favorite?.path ?? '',
			databasePath: s.database?.path ?? '',
			databaseType: s.database?.type ?? BLANK.databaseType,
			channelPath: s.channel?.path ?? '',
			channelType: s.channel?.type ?? BLANK.channelType
		}
		picked = undefined
		loading = false
	}
</script>

{#if import.meta.env.DEV}
	<div class="max-w-6xl mx-auto px-6 py-8 flex flex-col gap-8">
		<div class="flex flex-col gap-1">
			<h1 class="text-lg font-semibold text-emphasis">Composer suggestions</h1>
			<p class="text-xs text-secondary">
				The pills <code>composerSuggestions(signals, role)</code> returns. A blank path means no such
				item. Invitation prompts, which replace all of these, are not simulated.
			</p>
		</div>

		<div class="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-8">
			<div class="flex flex-col gap-6">
				<section class="flex flex-col gap-2">
					<h2 class="text-xs font-semibold text-emphasis">Role</h2>
					<ToggleButtonGroup bind:selected={roleName} wrap>
						{#snippet children({ item })}
							<ToggleButton value="operator" label="Operator" {item} />
							<ToggleButton value="developer" label="Developer" {item} />
							<ToggleButton value="workspaceAdmin" label="Workspace admin" {item} />
							<ToggleButton value="instanceAdmin" label="Instance admin" {item} />
						{/snippet}
					</ToggleButtonGroup>
					<p class="text-2xs text-hint">An instance admin here is also a workspace admin.</p>
				</section>

				<section class="flex flex-col gap-2">
					<h2 class="text-xs font-semibold text-emphasis">Load a real workspace</h2>
					<div class="flex flex-row gap-2 items-center">
						<Select
							class="grow"
							items={$userWorkspaces.map((w) => ({ label: `${w.name} (${w.id})`, value: w.id }))}
							bind:value={workspace}
							placeholder="Workspace"
						/>
						<Button
							variant="default"
							unifiedSize="md"
							startIcon={{ icon: Download }}
							disabled={!workspace}
							{loading}
							onClick={loadWorkspace}
						>
							Load signals
						</Button>
					</div>
					<p class="text-2xs text-hint">
						Fills the form with what the starter would read there. Answers are reused for 30 s.
					</p>
				</section>

				<section class="flex flex-col gap-2">
					<h2 class="text-xs font-semibold text-emphasis">Presets</h2>
					<div class="flex flex-row flex-wrap gap-2">
						{#each Object.keys(PRESETS) as name (name)}
							<Button variant="subtle" unifiedSize="sm" onClick={() => applyPreset(name)}>
								{name}
							</Button>
						{/each}
						<Button variant="subtle" unifiedSize="sm" onClick={() => applyPreset('')}>Clear</Button>
					</div>
				</section>

				<section class="flex flex-col gap-3">
					<h2 class="text-xs font-semibold text-emphasis">Signals</h2>
					<label class="flex flex-col gap-1">
						<span class="text-xs text-primary">Items</span>
						<TextInput
							bind:value={() => form.itemCount, (v) => (form.itemCount = String(v ?? ''))}
							size="sm"
							inputProps={{ type: 'number', min: 0, placeholder: 'absent' }}
						/>
						<span class="text-2xs text-hint">0 is empty, under 10 is new; the list reads 25</span>
					</label>
					{#each PATH_FIELDS as field (field.key)}
						<label class="flex flex-col gap-1">
							<span class="text-xs text-primary">{field.label}</span>
							<TextInput
								bind:value={() => form[field.key], (v) => (form[field.key] = String(v ?? ''))}
								size="sm"
								inputProps={{ placeholder: 'none' }}
							/>
						</label>
					{/each}
					<div class="flex flex-col gap-1">
						<span class="text-xs text-primary">Database (type and path)</span>
						<div class="flex flex-row gap-2">
							<Select class="w-40" items={DATABASE_ITEMS} bind:value={form.databaseType} />
							<TextInput
								class="grow"
								bind:value={form.databasePath}
								size="sm"
								inputProps={{ placeholder: 'none' }}
							/>
						</div>
					</div>
					<div class="flex flex-col gap-1">
						<span class="text-xs text-primary">Channel (type and path)</span>
						<div class="flex flex-row gap-2">
							<Select class="w-40" items={CHANNEL_ITEMS} bind:value={form.channelType} />
							<TextInput
								class="grow"
								bind:value={form.channelPath}
								size="sm"
								inputProps={{ placeholder: 'none' }}
							/>
						</div>
					</div>
				</section>
			</div>

			<div class="flex flex-col gap-6">
				<section class="flex flex-col gap-2">
					<h2 class="text-xs font-semibold text-emphasis">Pills</h2>
					<div class="rounded-md border border-border-light bg-surface p-3 min-h-12">
						{#if suggestions.length}
							<ComposerSuggestions {suggestions} onPick={(p) => (picked = p)} />
						{:else}
							<p class="text-xs text-hint">No pills.</p>
						{/if}
					</div>
					{#if picked}
						<div class="flex flex-col gap-1">
							<span class="text-2xs text-hint">Clicking that pill puts this in the box:</span>
							<p class="text-xs text-primary whitespace-pre-wrap">{picked}</p>
						</div>
					{/if}
				</section>

				<section class="flex flex-col gap-2">
					<h2 class="text-xs font-semibold text-emphasis">In order</h2>
					<ol class="flex flex-col gap-3">
						{#each suggestions as s, i (s.label)}
							<li class="flex flex-col gap-1">
								<div class="flex flex-row gap-2 items-center">
									<span class="text-2xs text-hint w-3">{i + 1}</span>
									<Badge color={s.mode === 'use' ? 'blue' : 'green'}>{s.mode}</Badge>
									<span class="text-xs font-medium text-emphasis">{s.label}</span>
								</div>
								<p class="text-2xs text-secondary pl-5">{s.prompt}</p>
							</li>
						{/each}
					</ol>
				</section>

				<section class="flex flex-col gap-2">
					<h2 class="text-xs font-semibold text-emphasis">Signals passed</h2>
					<pre class="text-2xs text-secondary bg-surface-secondary rounded-md p-3 overflow-x-auto"
						>{JSON.stringify(signals, null, 2)}</pre
					>
				</section>
			</div>
		</div>
	</div>
{/if}

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
		composerSuggestions,
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
		failedRuns: string
		waiting: string
		suspended: string
		liveWorkers: string
		errorHandler: 'absent' | 'set' | 'unset'
		draftPath: string
		deployPath: string
	}
	const BLANK: Form = {
		itemCount: '',
		failedRuns: '',
		waiting: '',
		suspended: '',
		liveWorkers: '',
		errorHandler: 'absent',
		draftPath: '',
		deployPath: ''
	}
	const PRESETS: Record<string, Partial<Form>> = {
		'Empty workspace': { itemCount: '0', liveWorkers: '3', errorHandler: 'unset' },
		Quiet: {
			itemCount: '10',
			failedRuns: '0',
			waiting: '0',
			suspended: '0',
			liveWorkers: '3',
			errorHandler: 'set'
		},
		'On fire': {
			itemCount: '10',
			failedRuns: '14',
			waiting: '120',
			suspended: '2',
			liveWorkers: '0',
			errorHandler: 'unset'
		},
		'Own work': {
			itemCount: '10',
			failedRuns: '0',
			liveWorkers: '3',
			errorHandler: 'unset',
			draftPath: 'f/billing/sync_invoices',
			deployPath: 'u/admin/etl_nightly'
		},
		'Signals failed': { itemCount: '10' }
	}

	let roleName = $state<RoleName>('instanceAdmin')
	let form = $state<Form>({ ...BLANK, ...PRESETS['Quiet'] })
	let workspace = $state<string | undefined>($workspaceStore ?? undefined)
	let loading = $state(false)
	let loadError = $state<string>()
	let picked = $state<string>()

	const num = (s: string) => (s.trim() === '' || isNaN(Number(s)) ? undefined : Number(s))

	const item = (kind: string, path: string) =>
		path.trim() ? { kind, path: path.trim(), name: path.trim().split('/').pop()! } : undefined

	const signals: ComposerSignals = $derived.by(() => {
		const waiting = num(form.waiting)
		const suspended = num(form.suspended)
		return {
			itemCount: num(form.itemCount),
			failedRuns: num(form.failedRuns),
			queue:
				waiting === undefined && suspended === undefined
					? undefined
					: { waiting: waiting ?? 0, suspended: suspended ?? 0 },
			liveWorkers: num(form.liveWorkers),
			hasErrorHandler: form.errorHandler === 'absent' ? undefined : form.errorHandler === 'set',
			myDraft: item('flow', form.draftPath),
			failedDeploy: item('script', form.deployPath)
		}
	})
	const suggestions = $derived(composerSuggestions(signals, ROLES[roleName]))

	function applyPreset(name: string) {
		form = { ...BLANK, ...PRESETS[name] }
		picked = undefined
	}

	const str = (n: number | undefined) => (n === undefined ? '' : String(n))

	// Reads with the selected role, so a role the logged-in user does not hold gets the
	// requests it would send refused, and those signals come back blank.
	async function loadWorkspace() {
		if (!workspace) return
		loading = true
		loadError = undefined
		try {
			const s = await readComposerSignals(workspace, ROLES[roleName])
			form = {
				itemCount: str(s.itemCount),
				failedRuns: str(s.failedRuns),
				waiting: str(s.queue?.waiting),
				suspended: str(s.queue?.suspended),
				liveWorkers: str(s.liveWorkers),
				errorHandler:
					s.hasErrorHandler === undefined ? 'absent' : s.hasErrorHandler ? 'set' : 'unset',
				draftPath: s.myDraft?.path ?? '',
				deployPath: s.failedDeploy?.path ?? ''
			}
			picked = undefined
		} catch (e) {
			loadError = String(e)
		} finally {
			loading = false
		}
	}

	type NumberKey = Exclude<keyof Form, 'errorHandler' | 'draftPath' | 'deployPath'>
	const NUMBER_FIELDS: { key: NumberKey; label: string; hint: string }[] = [
		{ key: 'itemCount', label: 'Items', hint: '0 is empty; the list reads at most 10' },
		{ key: 'failedRuns', label: 'Failed runs (24h)', hint: 'everyone' },
		{ key: 'waiting', label: 'Jobs waiting', hint: 'pill from 10' },
		{ key: 'suspended', label: 'Jobs suspended', hint: 'pill from 1' },
		{ key: 'liveWorkers', label: 'Live workers (5 min)', hint: 'instance admin; pill at 0' }
	]
</script>

{#if import.meta.env.DEV}
	<div class="max-w-6xl mx-auto px-6 py-8 flex flex-col gap-8">
		<div class="flex flex-col gap-1">
			<h1 class="text-lg font-semibold text-emphasis">Composer suggestions</h1>
			<p class="text-xs text-secondary">
				The pills <code>composerSuggestions(signals, role)</code> returns. A blank field is a signal
				that failed or that the role may not read. Invitation prompts, which replace all of these, are
				not simulated.
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
					{#if loadError}
						<p class="text-2xs text-red-500">{loadError}</p>
					{/if}
					<p class="text-2xs text-hint">
						Fills the form with what the starter would read there for the role above. Answers are
						reused for 30 s.
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
					<div class="grid grid-cols-2 gap-3">
						{#each NUMBER_FIELDS as field (field.key)}
							<label class="flex flex-col gap-1">
								<span class="text-xs text-primary">{field.label}</span>
								<TextInput
									bind:value={() => form[field.key], (v) => (form[field.key] = String(v ?? ''))}
									size="sm"
									inputProps={{ type: 'number', min: 0, placeholder: 'absent' }}
								/>
								<span class="text-2xs text-hint">{field.hint}</span>
							</label>
						{/each}
					</div>
					<div class="flex flex-col gap-1">
						<span class="text-xs text-primary">Error handler (workspace admins)</span>
						<ToggleButtonGroup bind:selected={form.errorHandler}>
							{#snippet children({ item })}
								<ToggleButton value="absent" label="Absent" {item} />
								<ToggleButton value="set" label="Set" {item} />
								<ToggleButton value="unset" label="Not set" {item} />
							{/snippet}
						</ToggleButtonGroup>
					</div>
					<label class="flex flex-col gap-1">
						<span class="text-xs text-primary">My latest draft (path)</span>
						<TextInput bind:value={form.draftPath} size="sm" inputProps={{ placeholder: 'none' }} />
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-xs text-primary">Latest failed deploy (path)</span>
						<TextInput
							bind:value={form.deployPath}
							size="sm"
							inputProps={{ placeholder: 'none' }}
						/>
					</label>
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

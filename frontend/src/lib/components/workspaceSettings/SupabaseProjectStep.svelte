<script lang="ts">
	import Alert from '../common/alert/Alert.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Password from '../Password.svelte'
	import Select from '../select/Select.svelte'
	import { Database, Loader2, Plus } from 'lucide-svelte'
	import { tick } from 'svelte'
	import { resource } from 'runed'
	import { sendUserToast } from '$lib/toast'
	import SupabaseConnectionMode from './SupabaseConnectionMode.svelte'
	import type { WizardState } from './addDataTableModel'
	import {
		getSupabaseOrgPlan,
		listSupabaseOrgs,
		listSupabaseProjects,
		orgSlug,
		projectOrg,
		projectRef,
		SUPABASE_REGIONS,
		type SupabaseOrg,
		type SupabaseProject
	} from './supabaseProvisioning'

	type Props = {
		/** The wizard's Supabase slice. Collected only -- nothing here creates anything. */
		intent: WizardState['supabase']
		token: string
		/** Hides the create tab where provisioning a billed project is not on offer. */
		existingOnly?: boolean
		/** Fired whenever the choice changes, so the host can drop what it derived from it. */
		onIntentChange?: () => void
	}

	let { intent = $bindable(), token, existingOnly = false, onIntentChange }: Props = $props()

	let orgs: SupabaseOrg[] | undefined = $state(undefined)
	let projects: SupabaseProject[] | undefined = $state(undefined)
	let plans: Record<string, string> = $state({})

	const listings = resource(
		() => token,
		async (t) => {
			if (!t) return
			try {
				orgs = await listSupabaseOrgs(t)
				projects = await listSupabaseProjects(t)
				// Someone who already has a Supabase database almost always means to connect it
				// rather than make a second one, so the step opens on the first of them. Decided
				// before anything renders, so no card visibly selects itself under the user.
				// Only seeds a choice that has not been made: this step is unmounted whenever the
				// wizard moves off it, so a user who picked "New project" and pressed Back would
				// otherwise come back to the first existing project instead.
				if (!intent.project && intent.mode !== 'create') {
					if (projects?.length) intent.project = projects[0]
					else if (!existingOnly) intent.mode = 'create'
				}
				// Seeded from the project, the way picking one does. Chosen independently, the
				// review step names whichever organization happens to be first while the database
				// under it belongs to another. A lookup that misses leaves it unset rather than
				// falling back to the first: `supabaseSummary` then shows the project's own
				// organization by identifier, which is right where a name would be wrong.
				const seeded = intent.project
				if (!intent.org) {
					intent.org = seeded
						? (orgs ?? []).find((o) => orgSlug(o) === projectOrg(seeded))
						: orgs?.[0]
				}
				// The plan decides who gets billed, and the list endpoint does not carry it.
				for (const o of orgs ?? []) {
					getSupabaseOrgPlan(t, orgSlug(o)).then((p) => {
						if (p) plans[orgSlug(o)] = p
					})
				}
			} catch (err) {
				sendUserToast(String(err), true)
				orgs = orgs ?? []
			}
		}
	)
	// Nothing but a spinner until *both* lists are in. Which mode to open on depends on the
	// projects, so clearing this when only the orgs have landed is what makes the toggle flip
	// under the user a moment later -- so it tracks the whole fetch, not each call in it.
	let loading = $derived(listings.loading)

	/** Supabase statuses are SCREAMING_SNAKE; only surface one that is not the happy path. */
	function projectStatus(p: SupabaseProject): string | undefined {
		if (!p.status || p.status === 'ACTIVE_HEALTHY') return undefined
		return p.status === 'INACTIVE' ? 'paused' : p.status.toLowerCase().replace(/_/g, ' ')
	}

	// Exclusive with the project cards, and each owns what it produced: a picked project, and
	// whatever the host derived from it, must not survive into the project about to exist.
	function selectNewProject() {
		if (intent.mode === 'create') return
		intent.mode = 'create'
		intent.project = undefined
		intent.password = ''
		onIntentChange?.()
	}

	/** Takes the picked project as a parameter: read directly off the prop inside a `$derived`,
	 * the checker narrows its optional type to `never`. */
	function isSelected(picked: SupabaseProject | undefined, p: SupabaseProject): boolean {
		return !!picked && projectRef(picked) === projectRef(p)
	}

	// The password field lives inside the card it belongs to, so it is scoped to one project:
	// carrying a value over to another card would show it already filled in. Selecting the last
	// card in a long list also grows it past the fold, hence the scroll once it has resized.
	async function selectProject(p: SupabaseProject, card: HTMLElement | null) {
		if (!isSelected(intent.project, p)) intent.password = ''
		intent.mode = 'existing'
		intent.project = p
		// So the review step can name the organization rather than print the project's slug.
		intent.org = (orgs ?? []).find((o) => orgSlug(o) === projectOrg(p)) ?? intent.org
		onIntentChange?.()
		await tick()
		card?.scrollIntoView({ block: 'nearest' })
	}

	/** Built from parameters rather than read off the surrounding `$state(undefined)`, which a
	 * `$derived` in the same scope narrows to `never`. */
	function orgOptions(
		all: SupabaseOrg[] | undefined,
		projs: SupabaseProject[] | undefined,
		plansBySlug: Record<string, string>
	) {
		return (all ?? []).map((o) => {
			const slug = orgSlug(o)
			const count = (projs ?? []).filter((p) => projectOrg(p) === slug).length
			return {
				label: o.name,
				value: slug,
				subtitle: [plansBySlug[slug], `${count} project${count === 1 ? '' : 's'}`]
					.filter(Boolean)
					.join(' · ')
			}
		})
	}

	let orgItems = $derived(orgOptions(orgs, projects, plans))
</script>

{#if loading}
	<div class="flex items-center gap-2 text-xs text-secondary py-2">
		<Loader2 size={16} class="animate-spin" />
		Loading your Supabase projects...
	</div>
{:else if (projects ?? []).length === 0 && existingOnly}
	<Alert type="info" size="xs" bgClass="border-0" title="">
		This Supabase account has no projects yet.
	</Alert>
{:else}
	{#if (projects ?? []).length}
		<span class="text-xs font-semibold text-emphasis">Projects in your Supabase account</span>
	{:else}
		<p class="text-xs text-secondary">This Supabase account has no projects yet.</p>
	{/if}
	<div class="flex flex-col gap-2 overflow-y-auto flex-1 min-h-24 pr-1">
		{#each projects ?? [] as p (projectRef(p))}
			{@const selected = intent.mode === 'existing' && isSelected(intent.project, p)}
			<!-- Not `RadioCard`: the selected card opens to hold a password field, and these carry
			a project icon and no radio dot. shrink-0 or the flex column squeezes the cards to
			fit instead of letting the list scroll, and the selected one loses its password
			field to the clip. -->
			<div
				class="shrink-0 border rounded-md overflow-hidden transition-colors {selected
					? 'border-border-selected/50 bg-surface-accent-selected'
					: 'border-border-light'}"
			>
				<button
					class="w-full text-left p-3 flex gap-3 items-start {selected
						? ''
						: 'hover:bg-surface-hover'}"
					onclick={(e) => selectProject(p, e.currentTarget.parentElement)}
				>
					<span class="mt-0.5 shrink-0"><Database size={18} class="text-secondary" /></span>
					<span class="flex flex-col gap-0.5 min-w-0">
						<span class="text-xs font-medium {selected ? 'text-accent' : 'text-emphasis'}"
							>{p.name}</span
						>
						<span class="text-xs text-secondary font-normal">
							{p.region}{projectOrg(p) ? ` · ${projectOrg(p)}` : ''}{projectStatus(p)
								? ` · ${projectStatus(p)}`
								: ''}
						</span>
					</span>
				</button>
				{#if selected}
					<div class="px-3 pb-3 flex flex-col gap-2">
						<div>
							<span class="text-xs font-semibold text-emphasis">Database password</span>
							<Password
								bind:password={
									() => intent.password, (v) => ((intent.password = v ?? ''), onIntentChange?.())
								}
								placeholder="••••••••"
							/>
							<p class="text-2xs text-secondary mt-1">
								Supabase only shows this when the project is created, and never exposes it through
								its API. If you no longer have it, <a
									href="https://supabase.com/dashboard/project/{projectRef(p)}/database/settings"
									target="_blank"
									rel="noreferrer"
									class="text-accent hover:underline">set a new one</a
								> — every existing connection to this project stops working when you do.
							</p>
						</div>
						<SupabaseConnectionMode bind:mode={intent.connectionMode} onChange={onIntentChange} />
					</div>
				{/if}
			</div>
		{/each}
		{#if !existingOnly}
			<div
				class="shrink-0 border rounded-md overflow-hidden transition-colors {intent.mode ===
				'create'
					? 'border-border-selected/50 bg-surface-accent-selected'
					: 'border-border-light'}"
			>
				<button
					class="w-full text-left p-3 flex gap-3 items-start {intent.mode === 'create'
						? ''
						: 'hover:bg-surface-hover'}"
					onclick={selectNewProject}
				>
					<span class="mt-0.5 shrink-0"><Plus size={18} class="text-secondary" /></span>
					<span class="flex flex-col gap-0.5 min-w-0">
						<span
							class="text-xs font-medium {intent.mode === 'create'
								? 'text-accent'
								: 'text-emphasis'}">New project</span
						>
						<span class="text-xs text-secondary font-normal"
							>Windmill creates it and stores its password</span
						>
					</span>
				</button>
				{#if intent.mode === 'create'}
					<div class="px-3 pb-3">{@render newProjectFields()}</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}

{#snippet newProjectFields()}
	<div class="flex flex-col gap-2">
		<div class="grid grid-cols-2 gap-2">
			<div>
				<span class="text-xs font-semibold text-emphasis">Organization</span>
				<!-- The list is keyed by slug because that is what the API takes; the whole
				organization is kept so the review step can name it. -->
				<Select
					items={orgItems}
					bind:value={
						() => (intent.org ? orgSlug(intent.org) : undefined),
						(v) => ((intent.org = (orgs ?? []).find((o) => orgSlug(o) === v)), onIntentChange?.())
					}
					placeholder={orgs === undefined ? 'Loading...' : 'Select'}
				/>
				<p class="text-2xs text-secondary mt-1">
					The project is created here and billed to this organization.
				</p>
			</div>
			<div>
				<span class="text-xs font-semibold text-emphasis">Region</span>
				<Select
					items={SUPABASE_REGIONS.map((r) => ({ label: r.label, value: r.code }))}
					bind:value={() => intent.region, (v) => ((intent.region = v), onIntentChange?.())}
					placeholder="Region"
				/>
			</div>
		</div>
		<div>
			<span class="text-xs font-semibold text-emphasis">Project name</span>
			<TextInput
				bind:value={
					() => intent.projectName, (v) => ((intent.projectName = String(v)), onIntentChange?.())
				}
				inputProps={{ placeholder: 'windmill-data' }}
			/>
		</div>
		<Alert type="info" size="xs" bgClass="border-0" title="">
			Windmill generates and stores the database password. A new project takes a minute or two to
			come up.
		</Alert>
		<SupabaseConnectionMode bind:mode={intent.connectionMode} onChange={onIntentChange} />
	</div>
{/snippet}

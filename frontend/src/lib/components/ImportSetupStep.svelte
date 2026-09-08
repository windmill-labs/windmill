<script lang="ts">
	import { ResourceService, WorkspaceService } from '$lib/gen'
	import { ArrowLeft, Check, Database, Loader2, TriangleAlert, X } from 'lucide-svelte'
	import { tick } from 'svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { Button } from '$lib/components/common'
	import AddDataTableWizard from '$lib/components/workspaceSettings/AddDataTableWizard.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import { SettingService } from '$lib/gen'
	import { resource } from 'runed'
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import ImportSetupRow from '$lib/components/ImportSetupRow.svelte'
	import AppConnectDrawer from '$lib/components/AppConnectDrawer.svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { applyRetarget, seesWholeWorkspace } from '$lib/importWizard/retargetDeployed'
	import { OauthService } from '$lib/gen'
	import { registryCcCapableFor } from '$lib/components/oauthRegistry'
	import { resourceTypeDisplayName } from '$lib/components/resourceTypeDisplay'
	import { applyOneMigration } from '$lib/components/workspaceSettings/projectInstall'
	import { probeMigrationsApplied } from '$lib/importWizard/probe'
	import {
		projectReferencesResource,
		retargetProjectExport,
		type ProjectExport,
		type ProjectMigration
	} from '$lib/components/workspaceSettings/projectBundle'
	import { superadmin, userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { escapeHtml } from '$lib/utils'

	// The last step, and the only optional one: it exists when the project's data
	// tables are not configured in the destination. The import has already run —
	// everything here is the part it could not do, because a data table is a named
	// database connection the workspace owns, not something an import can invent.
	//
	// Self-sufficient from `workspace` + `slug`: it re-fetches the export rather than
	// reading the executor, so reloading the page on this step still works. The plan
	// in the URL stays the whole state.

	interface Props {
		workspace: string
		slug: string
		/** The folder the import wrote into. The export names resources under the project's
		 *  own slug and `installProject` retargets them, so reading the raw paths here would
		 *  look for stubs that are not where they landed. */
		folder?: string
		onSkip: () => void
		onFinish: () => void
		onBack?: () => void
	}

	let { workspace, slug, folder, onSkip, onFinish, onBack }: Props = $props()

	type Row = {
		name: string
		migrations: ProjectMigration[]
		status: 'unconfigured' | 'running' | 'done' | 'failed' | 'unknown'
		error?: string
		/** Plays the confirmation flash once, right after the run that configured it. */
		justSaved: boolean
	}

	/** A resource the project shipped that needed filling in. */
	type Blank = {
		path: string
		resourceType: string
		/** Required fields the type declares and the value does not have yet. */
		missing: string[]
		/** Filled in since this step opened. The row stays — it is a checklist, and a
		 * line that vanishes when you complete it reads as something going wrong. */
		done: boolean
		/** Plays the confirmation flash once, right after the save that flipped it. */
		justSaved: boolean
		/**
		 * Something of another type already holds this path, named here. The import skipped it
		 * on the path alone, so this row exists to say the project did not get the resource it
		 * shipped — and to make sure nothing offers to write over what is there.
		 */
		occupiedBy?: string
		/**
		 * The resource could not be read, so nothing here knows whether it needs filling. Kept
		 * on the checklist rather than dropped: a read that fails is not evidence the resource
		 * is absent, and removing the row reports "all set" over a credential nobody filled.
		 */
		unreadable?: boolean
		/**
		 * The workspace resource this row was pointed at. The project's items reference it
		 * directly now, so this is what the row has to say instead of the path it used to name.
		 */
		reusedFrom?: string
		/**
		 * The empty placeholder is still at this row's path, because the retarget could not
		 * account for every item that might read it. Worth saying: the workspace has a resource
		 * on it that looks unfinished and is not.
		 */
		stubKept?: boolean
	}

	let loading = $state(true)
	let loadError = $state<string | undefined>(undefined)
	let rows = $state<Row[]>([])
	let blanks = $state<Blank[]>([])
	let projectResources: { path: string; resource_type: string }[] = []
	/**
	 * The subset of `projectResources` the checklist asks about: the ones something in the
	 * project actually points at. The rest are created and left alone — see
	 * `projectReferencesResource`. Kept apart from `projectResources` because the full list
	 * is still what a stub may not be replaced by.
	 */
	let askableResources: { path: string; resource_type: string }[] = []
	let working = $state(false)
	let resourceEditor: ResourceEditorDrawer | undefined = $state(undefined)

	/** The folder the import wrote into, which is where every rewritable referrer lives. */
	const targetFolder = $derived(folder?.trim() || slug)

	/**
	 * Resources the workspace already has, by resource type — what a stub can be replaced
	 * by. Empty for a workspace this import created, which is why the choice is offered
	 * rather than imposed: with nothing to choose from the button goes straight to the
	 * editor, exactly as it did before.
	 */
	let candidates = $state<Record<string, string[]>>({})
	/**
	 * How many candidates are worth reading back to find the unfilled ones. Past this a
	 * workspace holds too many resources of these types to be the case worth filtering —
	 * one project's stub offered as another's credential — and they are all offered rather
	 * than costing a request each.
	 */
	const CANDIDATE_READ_CAP = 40
	/** The credential row whose choice dialog is open. */
	let choosing = $state<Blank | undefined>(undefined)
	let chosenPath = $state<string | undefined>(undefined)
	let reusing = $state(false)

	const pendingTables = $derived(rows.filter((r) => r.status !== 'done'))
	// Split because the two say different things to the user: one data table was never
	// created, the other exists and could not be read. Telling someone to set up what they
	// have already set up is how a warning stops being believed.
	// Grouped by what it costs the project, not by row status. A row whose migrations failed is
	// configured, but its tables are as absent as one that was never created and the remedy is
	// the same — so they share a message. An `unknown` row is the odd one: it is set up, and
	// whether its tables are there is precisely what could not be established.
	const missingTables = $derived(
		rows.filter((r) => r.status === 'unconfigured' || r.status === 'failed')
	)
	const uncheckedTables = $derived(rows.filter((r) => r.status === 'unknown'))
	/** Rows the user has not dealt with, of either kind. */
	const outstanding = $derived(pendingTables.length + blanks.filter((b) => !b.done).length)

	// The wizard needs the instance-database pool and a confirmation host; the settings
	// page owns them there, so this step owns them here.
	// Which resource types this instance has an OAuth client for. A resource whose type is
	// in here can be connected instead of hand-filled, which for an OAuth type is the
	// difference between clicking Connect and pasting a token that expires in an hour.
	// Empty when no superadmin has configured any client — then every row falls back to
	// the editor, which is the only thing that would work anyway.
	const oauthConnects = resource(
		() => workspace,
		async () => {
			try {
				return (await OauthService.listOauthConnects()).map((c) => c.name)
			} catch {
				return []
			}
		}
	)
	const instanceConnects = $derived(new Set(oauthConnects.current ?? []))
	/**
	 * Matches what the dialog itself decides (`AppConnectInner.open`: `manual = !inConnects &&
	 * !registryCcCapable()`). An instance client is the usual route, but a provider the
	 * registry marks client-credentials-capable is connectable without one, because those
	 * credentials are entered per resource rather than held by a superadmin. Test only the
	 * first half and Connect disappears on the eight such providers, where it would work.
	 */
	const canConnectType = (rt: string) => instanceConnects.has(rt) || registryCcCapableFor(rt)
	let appConnect: AppConnectDrawer | undefined = $state(undefined)

	const customInstanceDbs = resource([() => workspace], SettingService.listCustomInstanceDbs)
	const confirmationModal = createAsyncConfirmationModal()
	let wizardOpen = $state(false)
	let wizard = $state<AddDataTableWizard | undefined>(undefined)
	let wizardFor = $state<string | undefined>(undefined)
	let configuredNames = $state<{ name: string; resourcePath: string | undefined }[]>([])

	/**
	 * Which row the open dialog runs migrations for. Separate from `wizardFor`, which
	 * `afterWizard()` clears as soon as the run reports — while the dialog stays up offering
	 * "Try again" for exactly that row. Read by `onFinishAlso`, so it has to outlive the run
	 * rather than the opening.
	 */
	let retryTarget = $state<string | undefined>(undefined)

	/**
	 * A dialog is being opened. `open()` resolves the destination membership before it shows
	 * anything, so the button stays clickable during that wait — and a second click starts a
	 * second lookup whose `reset()` lands on the dialog the first one opened, wiping fields
	 * the user has already filled.
	 */
	let opening = $state(false)

	function openWizard(name: string) {
		wizardFor = name
		retryTarget = name
		// `open()`, not `opened = true`: only the method runs the wizard's own reset, which
		// is what applies `initialName` and clears whatever a previous run left behind.
		// `wizardOpen` is deliberately not set here. `open()` sets it once it has resolved the
		// destination's membership, and it is bound to the dialog's `opened` — so setting it
		// now shows a live, clickable dialog while that lookup is still in flight, with the
		// username unresolved. Setup reached in that window writes the credential path this
		// whole chain exists to get right. `wizardFor` alone mounts the component, which is
		// all `wizard?.open()` needs to exist.
		opening = true
		void tick()
			.then(() => wizard?.open())
			.finally(() => (opening = false))
	}

	function defaultInstanceDbName(): string {
		const used = Object.keys(customInstanceDbs.current ?? {})
		let n = 1
		while (used.includes(`dt${n}`)) n++
		return `dt${n}`
	}

	/**
	 * What a row's state actually is, asked of the destination rather than inferred.
	 *
	 * The data table existing is not the question — the wizard creates it and the migrations
	 * run afterwards, so a table can be there with none of the project's tables inside it.
	 * That gap is invisible in memory after a reload, which rebuilds every row from scratch;
	 * reading it as "done" would let the step say "You're all set" over a project whose apps
	 * all fail on open.
	 *
	 * `probeMigrationsApplied` answers `undefined` when it cannot tell — an unreadable schema
	 * (the data table's database is down, or its credentials have gone bad) or SQL naming no
	 * tables it can resolve. That is neither done nor missing, and claiming either goes beyond
	 * what the code knows: reading it as done is how this step ends up reporting "You're all
	 * set" over a project whose apps fail on open. `unknown` says what is true, and still
	 * counts as outstanding so nothing is finished on top of it.
	 */
	async function settle(
		name: string,
		ms: ProjectMigration[],
		absent: boolean,
		prev: Row | undefined
	): Promise<Row['status']> {
		if (absent) return 'unconfigured'
		const applied = await probeMigrationsApplied(workspace, name, ms)
		if (applied === true) return 'done'
		if (applied === false) return prev?.status === 'failed' ? 'failed' : 'unconfigured'
		return prev?.status === 'failed' ? 'failed' : 'unknown'
	}

	/** Which data tables the project needs that the destination does not have yet. */
	async function load() {
		loading = true
		loadError = undefined
		try {
			const res = await fetch(
				`/api/w/${encodeURIComponent(workspace)}/hub/projects/${encodeURIComponent(slug)}/export`
			)
			if (!res.ok) throw new Error(`the hub proxy answered ${res.status}`)
			const exportData = (await res.json()) as ProjectExport
			const enabled = (exportData.migrations ?? []).filter(
				(m) => m.enabled && (m.sql ?? '').trim() !== ''
			)
			// Kept, not just counted: which data tables the destination has decides whether a
			// row can retry its migrations or has to go back through the wizard, and after a
			// reload this call is the only thing that knows. Drop it and such a row offers the
			// wizard, which then refuses the name it created itself.
			const tables = await WorkspaceService.listDataTables({ workspace })
			configuredNames = tables.map((t) => ({ name: t.name, resourcePath: t.resource_path }))
			const present = new Set(tables.map((d) => d.name))
			const missing = [...new Set(enabled.map((m) => m.datatable_name))].filter(
				(n) => !present.has(n)
			)
			const previous = new Map(rows.map((r) => [r.name, r]))
			rows = await Promise.all(
				[...new Set(enabled.map((m) => m.datatable_name))].map(async (name) => {
					const ms = enabled.filter((m) => m.datatable_name === name)
					const prev = previous.get(name)
					const status = await settle(name, ms, missing.includes(name), prev)
					return prev
						? { ...prev, migrations: ms, status, error: status === 'done' ? undefined : prev.error }
						: { name, migrations: ms, status, justSaved: false }
				})
			)
			// Retargeted the same way the import was, so these are where the stubs actually
			// landed. `retargetProjectExport` is a no-op when the folder is the slug, which is
			// every new-workspace import.
			const target = targetFolder
			const retargeted = retargetProjectExport(exportData, exportData.project?.slug ?? slug, target)
			// Contained for the same reason the import contains: a crafted export can name a
			// path outside the folder, and offering that for editing would reach a resource
			// this import was never allowed to create.
			projectResources = (retargeted.resources ?? [])
				.map((r) => ({ path: String(r.path), resource_type: String((r as any).resource_type) }))
				.filter((r) => r.path.startsWith(`f/${target}/`))
			// Asked against the export as published, not the retargeted copy: a path the project
			// spells out in code is not rewritten by the retarget, so only the raw export has
			// its references and its resource paths agreeing. `resourceCount` asks the same
			// question the same way, and the step and the stepper have to give one answer.
			// Paired by position, not by reconstructing the retargeted path: `retargetProjectExport`
			// maps `resources` in order, and an external path the bundle pulled in lands at
			// `f/<folder>/<name>` with a `_2` suffix on collision, which no slicing recovers.
			const askable = new Set(
				(retargeted.resources ?? [])
					.map((r, i) => [String(r.path), (exportData.resources ?? [])[i]] as const)
					.filter(([, raw]) => raw && projectReferencesResource(exportData, String(raw.path)))
					.map(([path]) => path)
			)
			askableResources = projectResources.filter((r) => askable.has(r.path))
			await refreshBlanks()
		} catch (e: any) {
			loadError = e?.body ?? e?.message ?? String(e)
		} finally {
			loading = false
		}
	}

	/**
	 * Resources the import created but could not fill. Every shipped resource arrives
	 * as a stub — the hub never publishes resource values, they are credentials — so
	 * this is not "which ones are empty" but "which ones are *still* empty": a
	 * re-import leaves an already-filled resource alone.
	 *
	 * The type's schema names the required fields, so the row can say what is missing
	 * rather than just that something is. A type we cannot read still counts as blank
	 * when the value is empty; it just lists no field names.
	 */
	async function findBlankResources(
		resources: { path: string; resource_type: string }[]
	): Promise<Blank[]> {
		const out: Blank[] = []
		for (const r of resources) {
			let value: any
			let occupiedBy: string | undefined
			try {
				const found = await ResourceService.getResource({ workspace, path: r.path })
				value = found?.value
				// The presence probe matches on path, and a path says nothing about type. A
				// resource of another kind sitting here is not this project's stub, however
				// empty it looks — treating it as one offers to fill somebody else's resource.
				if (found?.resource_type && found.resource_type !== r.resource_type) {
					occupiedBy = found.resource_type
				}
			} catch (e: any) {
				// A 404 is the import having failed to create it, which it reported itself.
				// Any other failure is a read this could not complete, which says nothing about
				// whether the resource is there or needs filling — so the row stays.
				if (e?.status === 404) continue
				out.push({
					path: r.path,
					resourceType: r.resource_type,
					missing: [],
					done: false,
					justSaved: false,
					unreadable: true
				})
				continue
			}
			const filled = new Set(
				value && typeof value === 'object'
					? Object.entries(value)
							.filter(([, v]) => v !== undefined && v !== null && v !== '')
							.map(([k]) => k)
					: []
			)
			let required: string[] = []
			// A type whose schema will not load leaves `required` empty, which reads as "nothing
			// missing" — and a half-filled resource would drop off the checklist as done. The
			// row is kept instead; it just cannot name which fields are short.
			let requirementsUnknown = false
			try {
				const schema = (await ResourceService.getResourceType({ workspace, path: r.resource_type }))
					?.schema as { required?: string[] } | undefined
				required = schema?.required ?? []
			} catch {
				requirementsUnknown = true
			}
			const missing = required.filter((k) => !filled.has(k))
			// A conflicting occupant is always listed, however full its value looks: the row is
			// what tells the user the project is missing a resource it shipped.
			if (occupiedBy || requirementsUnknown || missing.length > 0 || filled.size === 0) {
				out.push({
					path: r.path,
					resourceType: r.resource_type,
					missing,
					done: false,
					justSaved: false,
					occupiedBy
				})
			}
		}
		return out
	}

	/**
	 * Re-read the resources and settle each row's state. Rows are never dropped once
	 * listed: the first pass decides what the checklist contains, and every pass after
	 * it only moves a row from outstanding to done.
	 */
	async function refreshBlanks(): Promise<void> {
		const fresh = await findBlankResources(askableResources)
		const stillBlank = new Map(fresh.map((b) => [b.path, b]))
		if (blanks.length === 0) {
			blanks = fresh
			await loadCandidates()
			return
		}
		blanks = blanks.map((b) => {
			// A row pointed at another resource is settled once its own stub is gone: the
			// project's items read the chosen resource and nothing is left at this path. A row
			// whose stub was kept is not settled, and re-reading it is how filling that stub in
			// finally closes the row.
			if (b.reusedFrom && !b.stubKept) return b
			const f = stillBlank.get(b.path)
			// Every field the fresh read decides is taken from it, not merged selectively: these
			// describe what is at the path *now*. Keeping a stale `unreadable` leaves a resource
			// that has since come back blocked until a reload, and keeping a stale absence hides
			// one that has just become unreadable.
			if (f) {
				return {
					...b,
					missing: f.missing,
					unreadable: f.unreadable,
					occupiedBy: f.occupiedBy,
					done: false,
					justSaved: false
				}
			}
			// Gone from the blank list entirely: it was read, and it is filled. `stubKept` goes
			// with it — the placeholder the items this run could not move read is a credential
			// now, so there is nothing left to tell anyone to fill in.
			return {
				...b,
				missing: [],
				unreadable: undefined,
				occupiedBy: undefined,
				stubKept: undefined,
				done: true,
				justSaved: !b.done
			}
		})
		// The flash is a one-shot; clear it so a later refresh does not replay it.
		for (const b of blanks) {
			if (!b.justSaved) continue
			setTimeout(() => {
				const row = blanks.find((x) => x.path === b.path)
				if (row) row.justSaved = false
			}, 1500)
		}
		await loadCandidates()
	}

	/**
	 * Which existing resources each outstanding row could be replaced by. Re-read on every
	 * refresh rather than once: a resource created from the editor here is a candidate for
	 * the rows below it.
	 *
	 * The project's own resources are never offered — one of this project's stubs standing
	 * in for another is a reference to something equally unfilled.
	 */
	async function loadCandidates(): Promise<void> {
		const types = [...new Set(blanks.map((b) => b.resourceType))]
		if (types.length === 0) {
			candidates = {}
			return
		}
		const own = new Set(projectResources.map((r) => r.path))
		const next: Record<string, string[]> = Object.fromEntries(types.map((t) => [t, []]))
		try {
			// One call for every type at once — `resource_type` takes a comma-separated list —
			// and every page of it: `perPage` is what bounds the answer, so without the loop a
			// workspace past one page would have the rest of its resources silently hidden.
			for (let page = 1; page <= 100; page++) {
				const rows = await ResourceService.listResource({
					workspace,
					resourceType: types.join(','),
					page,
					perPage: 100
				})
				for (const r of rows) {
					if (own.has(r.path)) continue
					next[r.resource_type ?? '']?.push(r.path)
				}
				if (rows.length < 100) break
			}
		} catch {
			// Offer nothing rather than a partial list: every row then behaves as it did before
			// this choice existed, which is a working way to fill a credential.
			candidates = {}
			return
		}
		// An unfilled resource is never the answer to "which credential should this use" —
		// another project's stub above all, which the path filter above cannot recognise.
		const paths = Object.values(next).flat()
		if (paths.length <= CANDIDATE_READ_CAP) {
			const settled = await Promise.all(paths.map(async (p) => [p, await isUnfilled(p)] as const))
			const unfilled = new Set(settled.filter(([, empty]) => empty).map(([p]) => p))
			for (const t of Object.keys(next)) next[t] = next[t].filter((p) => !unfilled.has(p))
		}
		candidates = next
	}

	/**
	 * Whether a resource holds nothing. Same test the checklist uses to call one of the
	 * project's own resources blank, so a resource this drops is exactly one the wizard
	 * would have asked someone to fill in.
	 */
	async function isUnfilled(path: string): Promise<boolean> {
		try {
			const found = await ResourceService.getResource({ workspace, path })
			const value = found?.value
			if (!value || typeof value !== 'object') return true
			return !Object.values(value).some((v) => v !== undefined && v !== null && v !== '')
		} catch {
			// A read that fails says nothing about the value, and offering it is what this did
			// before the check existed.
			return false
		}
	}

	/**
	 * The row's one action. A workspace that already has a resource of this type gets the
	 * choice first — reusing what is there is usually the answer, and entering the same
	 * credentials a second time is the thing worth avoiding. With nothing to choose from
	 * there is no choice to make, so it goes straight where it always went.
	 */
	function startFilling(b: Blank): void {
		// A kept-stub row has already been pointed at a resource; what is left is the empty
		// placeholder the items this run could not move still read. Reusing a second resource
		// would move nothing — every rewritable referrer is off the stub — and would relabel
		// the row after a retarget that did nothing.
		if (b.done || b.stubKept || (candidates[b.resourceType] ?? []).length === 0) {
			fillDirectly(b)
			return
		}
		chosenPath = undefined
		choosing = b
	}

	/** Connect where the instance can, hand-fill otherwise. */
	function fillDirectly(b: Blank): void {
		if (canConnectType(b.resourceType)) appConnect?.open(b.resourceType, b.path)
		else resourceEditor?.initEdit(b.path)
	}

	/**
	 * The chooser's way out: close it and do what the button did before there was a choice.
	 * The row is read out of the state first — closing the dialog unmounts the block that
	 * would otherwise be holding it.
	 */
	function fillNewInstead(): void {
		const b = choosing
		choosing = undefined
		if (b) fillDirectly(b)
	}

	/**
	 * Point the project at an existing resource: every imported item that referenced the stub
	 * is rewritten to the chosen path. Nothing is copied. The stub is deleted only when
	 * `applyRetarget` can account for every item that might read it, and kept otherwise — so
	 * the toast says how many items moved, and whether the placeholder is still there.
	 */
	async function reuseChosen(): Promise<void> {
		const b = choosing
		const target = chosenPath
		if (!b || !target) return
		reusing = true
		working = true
		try {
			const outcome = await applyRetarget({
				workspace,
				folder: targetFolder,
				from: b.path,
				to: target,
				// Asked of this workspace, not of whichever one the user record still describes:
				// reloading on this step leaves `$userStore` pointing at the previous workspace.
				seesWholeWorkspace: seesWholeWorkspace($userStore, !!$superadmin, workspace)
			})
			const moved = `${outcome.rewritten.length} item${outcome.rewritten.length === 1 ? '' : 's'}`
			if (outcome.error) {
				sendUserToast(
					`Could not point the project at ${target}: ${outcome.error}. ${moved} had already been updated, and ${b.path} was kept.`,
					true
				)
				return
			}
			choosing = undefined
			const row = blanks.find((x) => x.path === b.path)
			if (row) {
				row.reusedFrom = target
				row.stubKept = !outcome.stubDeleted
				// Settled only when the stub is gone. A kept stub is empty and is still what
				// every item the scan could not move reads, so the row stays outstanding and
				// keeps its action: filling it in is the thing left to do.
				row.done = outcome.stubDeleted
				row.justSaved = outcome.stubDeleted
			}
			await refreshBlanks()
			sendUserToast(
				outcome.stubDeleted
					? `The project now uses ${target} — ${moved} updated.`
					: `The project now uses ${target} — ${moved} updated. ${b.path} was kept, because some items could not be checked.`
			)
		} catch (e: any) {
			sendUserToast(
				`Could not point the project at ${target}: ${e?.body ?? e?.message ?? String(e)}`,
				true
			)
		} finally {
			reusing = false
			working = false
		}
	}

	$effect(() => {
		void load()
	})

	/**
	 * The data table now exists — run the migrations that were skipped for it during the
	 * import, which is the whole reason this step waits for the configuration.
	 */
	async function runMigrationsFor(name: string): Promise<void> {
		const row = rows.find((r) => r.name === name)
		// Thrown, not returned: this also runs as the wizard's appended step, which reads a
		// resolved promise as "the migrations ran". Resolving for a name that matches no row
		// would report success over SQL that never executed.
		if (!row) throw new Error(`No data table named '${name}' in this project`)
		working = true
		row.status = 'running'
		try {
			for (const m of row.migrations) await applyOneMigration(workspace, slug, m)
			row.status = 'done'
			row.error = undefined
			// One-shot, cleared by name rather than by reference: `load()` rebuilds the row
			// objects, so the one holding the flag when it fires may not be this one.
			row.justSaved = true
			setTimeout(() => {
				const current = rows.find((r) => r.name === name)
				if (current) current.justSaved = false
			}, 1500)
		} catch (e: any) {
			row.status = 'failed'
			row.error = e?.body ?? e?.message ?? String(e)
			sendUserToast(`Could not run the migrations for ${name}: ${row.error}`, true)
			// Rethrown, because this also runs as the wizard's last checklist step
			// (`onFinishAlso`). Swallowing it there makes the wizard report a clean finish
			// over a failed migration, and close — leaving the data table name taken and no
			// way back to retry it.
			throw e
		} finally {
			working = false
		}
	}

	/**
	 * Leaving a credential unfilled costs the project the parts that read it. Leaving a
	 * data table unconfigured costs it everything: the apps query tables that do not
	 * exist, so they fail on open rather than degrading. Only the second is worth
	 * stopping for, and the wizard is the only place that can still run the migration —
	 * nothing in the workspace knows the project shipped one.
	 */
	async function skip(): Promise<void> {
		if (pendingTables.length > 0) {
			// Escaped: `confirmationModal.ask` renders `children` through `createRawSnippet`,
			// so this string is HTML, and the name is a `datatable_name` straight out of the
			// hub export. A hub is not ours — `hub_base_url` is an instance setting and the
			// wizard can be pointed at any of them — so a name carrying an event-bearing
			// element would otherwise run script in this authenticated origin.
			// One block per outcome, the way the footer alert does it. A single sentence over a
			// mixed list has to be wrong about half of it: an `unknown` data table is set up —
			// only its schema could not be read — so naming it under "not set up" tells the user
			// to do something they have already done.
			const missing = missingTables
			const unverified = uncheckedTables
			const listOf = (rs: Row[]) => rs.map((r) => escapeHtml(r.name)).join(', ')
			const blocks: string[] = []
			if (missing.length > 0) {
				const one = missing.length === 1
				blocks.push(
					`The tables ${one ? 'the data table' : 'the data tables'} <b>${listOf(missing)}</b> ` +
						`${one ? 'holds' : 'hold'} do not exist, and this project's apps and flows read ` +
						`them. Every one of those fails as soon as it opens.<br /><br />` +
						`Setting ${one ? 'it' : 'them'} up later from workspace settings creates the ` +
						`connection but not the tables — only this step runs the project's migration.`
				)
			}
			if (unverified.length > 0) {
				const one = unverified.length === 1
				blocks.push(
					`${one ? 'The data table' : 'The data tables'} <b>${listOf(unverified)}</b> ` +
						`${one ? 'is' : 'are'} set up, but ${one ? 'its' : 'their'} schema could not be ` +
						`read, so whether this project's tables exist is unknown. Its apps and flows will ` +
						`fail wherever they query a table that is missing.`
				)
			}
			const confirmed = await confirmationModal.ask({
				title: missing.length > 0 ? 'The project will not run' : 'This has not been verified',
				confirmationText: 'Skip anyway',
				type: missing.length > 0 ? 'danger' : 'info',
				children: blocks.join('<br /><br />')
			})
			if (!confirmed) return
		}
		onSkip()
	}

	/**
	 * After the wizard closes. The migrations already ran inside its checklist, via
	 * `onFinishAlso`, so this only re-reads what exists now — including the case where
	 * the wizard was cancelled, or made a table under a different name than the row
	 * asked for, which leaves the row outstanding rather than falsely done.
	 */
	async function afterWizard(): Promise<void> {
		const name = wizardFor
		wizardFor = undefined
		try {
			const tables = await WorkspaceService.listDataTables({ workspace })
			configuredNames = tables.map((t) => ({ name: t.name, resourcePath: t.resource_path }))
			const present = new Set(tables.map((t) => t.name))
			const row = name ? rows.find((r) => r.name === name) : undefined
			if (row && row.status !== 'done' && row.status !== 'failed' && !present.has(name!)) {
				row.status = 'unconfigured'
			}
		} catch {
			// Nothing to correct with; the row keeps whatever the run left it saying.
		}
	}
</script>

<div class="flex flex-col gap-4">
	<div>
		<h2 class="text-sm font-semibold text-emphasis">Finish setting up</h2>
		<!-- Reads as what the user gets out of it, not as what the import failed to do:
		     the step is skippable, so it has to say why finishing is worth their time. -->
		<p class="mt-0.5 text-xs text-secondary">
			Your project is imported. For its apps and flows to actually run, they need a place to store
			data and credentials for the services they use — the import can't supply those for you.
		</p>
	</div>

	{#if loading}
		<div class="flex items-center gap-2 text-xs text-secondary">
			<Loader2 size={14} class="animate-spin" /> Checking what this project needs…
		</div>
	{:else if loadError}
		<Alert type="warning" title="Could not check the project's data tables" size="xs">
			{loadError}. You can finish and configure them later in Workspace settings → Data tables.
		</Alert>
	{:else}
		{#if rows.length > 0}
			<!-- Named and explained: the row underneath is a table called `main`, which
			     says nothing to someone meeting the concept for the first time. -->
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">
					Data table{rows.length === 1 ? '' : 's'} to set up ({rows.length})
				</span>
				<p class="text-xs font-normal text-secondary">
					Where apps and flows keep the data they read and write.
				</p>
			</div>
		{/if}
		<ul class="flex flex-col gap-1.5">
			{#each rows as row (row.name)}
				{@const sql = row.migrations
					.map((m) => m.sql)
					.filter(Boolean)
					.join('\n\n')}
				{@const hasTable = configuredNames.some((c) => c.name === row.name)}
				<ImportSetupRow flash={row.justSaved}>
					{#snippet icon()}
						{#if row.status === 'done'}
							<Check size={20} class="text-emerald-600" />
						{:else if row.status === 'running'}
							<Loader2 size={20} class="animate-spin text-blue-500" />
						{:else if row.status === 'failed'}
							<X size={20} class="text-red-500" />
						{:else if row.status === 'unknown'}
							<TriangleAlert size={20} class="text-yellow-600" />
						{:else}
							<Database size={20} class="text-secondary" />
						{/if}
					{/snippet}
					{#snippet title()}
						<span class="min-w-0 truncate font-mono text-emphasis">{row.name}</span>
					{/snippet}
					{#snippet detail()}
						<span class="truncate text-secondary">
							{#if row.status === 'done'}
								{row.migrations.length} migration{row.migrations.length === 1 ? '' : 's'} run
							{:else if row.status === 'running'}
								running migrations…
							{:else if row.status === 'failed'}
								<span class="text-red-500">{row.error}</span>
							{:else if row.status === 'unknown'}
								set up, but its tables could not be read — the database may be unreachable
							{:else}
								not configured yet
							{/if}
						</span>
					{/snippet}
					{#snippet extra()}
						<!-- The SQL, before anything runs it. Step 3 reviews the migrations it can
						     run there; the ones deferred to here were never shown, and "Set up"
						     executes them against whatever database the wizard is pointed at —
						     which can be one that already holds unrelated objects.

						     On an `unknown` row nothing here will run — its only action re-reads —
						     so the summary says what the SQL is rather than promising to run it. -->
						{#if sql && row.status !== 'done'}
							<details class="mt-1.5">
								<summary class="cursor-pointer text-2xs text-secondary hover:text-primary">
									{row.status === 'unknown'
										? 'Show the SQL this project ships'
										: 'Show the SQL this will run'}
								</summary>
								<pre
									class="mt-1.5 max-h-52 overflow-auto whitespace-pre-wrap rounded border border-border-light bg-surface-secondary p-2 font-mono text-2xs text-secondary"
									>{sql}</pre
								>
							</details>
						{/if}
					{/snippet}
					{#snippet action()}
						<!-- The wizard owns creating a data table: picking or provisioning the
						     database, writing the config, and reporting the connection. This step
						     only says which name it needs and runs the migrations afterwards. -->
						{#if row.status === 'unknown'}
							<!-- Never runs the SQL. `unknown` covers two different unknowns — the schema
							     could not be read, or the SQL names no table this can resolve — and the
							     second is arbitrary published SQL that may carry a non-idempotent INSERT
							     or ALTER. Applying it a second time on the chance it never applied once
							     is a worse outcome than saying so. Reading again is free and settles the
							     case that actually recovers: a database that was briefly unreachable. -->
							<Button
								variant="subtle"
								unifiedSize="sm"
								disabled={working || loading}
								onClick={() => void load()}
							>
								Check again
							</Button>
						{:else if hasTable && row.status !== 'done' && row.status !== 'running'}
							<!-- The data table is there and its tables are not, so the thing left
							     to do is run the migrations. Reopening the wizard would ask for a
							     name it now holds itself, which it rejects as taken — leaving no
							     way back to the step that actually failed.

							     Keyed on the data table existing rather than on the row saying
							     `failed`, because a reload rebuilds every row from scratch: the
							     same situation then reads as `unconfigured`, with nothing left in
							     memory to say a migration was ever attempted. -->
							<Button
								variant="accent"
								unifiedSize="sm"
								disabled={working}
								onClick={() => void runMigrationsFor(row.name).catch(() => {})}
							>
								{row.status === 'failed' ? 'Run migrations again' : 'Run migrations'}
							</Button>
						{:else}
							<!-- Everything the branches above do not claim: a configured row, one whose
							     migrations are running, and one whose data table does not exist yet. -->
							<Button
								variant={row.status === 'done' ? 'subtle' : 'accent'}
								unifiedSize="sm"
								disabled={working || opening}
								onClick={() => openWizard(row.name)}
							>
								{#if row.status === 'done'}
									Configured
								{:else if row.status === 'running'}
									Setting up…
								{:else}
									Set up
								{/if}
							</Button>
						{/if}
					{/snippet}
				</ImportSetupRow>
			{/each}
		</ul>

		{#if blanks.length > 0}
			<div class="flex flex-col gap-2">
				<span class="text-xs font-semibold text-emphasis"
					>Credentials to fill ({blanks.length})</span
				>
				<ul class="flex flex-col gap-1.5">
					{#each blanks as b (b.path)}
						{@const blocked = !!b.occupiedBy || !!b.unreadable}
						{@const canConnect = !b.done && !blocked && canConnectType(b.resourceType)}
						<!-- Laid out like the resource type rows in the Add-a-resource drawer: the
						     integration's own icon, its product name, and the raw identifier demoted
						     beside it. The path only matters when two resources share a type, so it
						     stops being the thing the eye lands on. -->
						<ImportSetupRow flash={b.justSaved}>
							{#snippet icon()}
								{#if b.done}
									<Check size={20} class="text-emerald-600" />
								{:else if blocked}
									<TriangleAlert size={20} class="text-yellow-600" />
								{:else}
									<IconedResourceType name={b.resourceType} silent width="20px" height="20px" />
								{/if}
							{/snippet}
							{#snippet title()}
								<div class="flex min-w-0 flex-row items-baseline gap-2">
									<span class="min-w-0 truncate text-emphasis">
										{resourceTypeDisplayName(b.resourceType)}
									</span>
									<span class="min-w-0 truncate font-mono text-2xs font-normal text-hint">
										{b.path}
									</span>
								</div>
							{/snippet}
							{#snippet detail()}
								{#if b.reusedFrom}
									<span class="truncate text-secondary">
										now uses <span class="font-mono">{b.reusedFrom}</span>
									</span>
									{#if b.stubKept}
										<!-- Not a footnote: some items were not moved and still read this path,
										     so the empty resource on it is a credential someone has to fill in. -->
										<span class="truncate text-hint">
											some items still read <span class="font-mono">{b.path}</span> — fill it in too
										</span>
									{/if}
								{:else if b.occupiedBy}
									<span class="truncate text-secondary">
										a {resourceTypeDisplayName(b.occupiedBy)} resource already holds this path — the
										project did not get this one
									</span>
								{:else if b.unreadable}
									<span class="truncate text-secondary">
										could not be read, so whether it needs filling is unknown
									</span>
								{:else if !b.done && b.missing.length > 0}
									<span class="truncate text-secondary">
										Missing {b.missing.join(', ')}
									</span>
								{/if}
							{/snippet}
							{#snippet action()}
								<!-- Connect where the instance has a client for this type: asking for an
								     OAuth resource by hand means pasting an access token that dies within
								     the hour, since only a token Windmill obtained itself gets refreshed. -->
								{#if blocked}
									<!-- No action: every one here writes to the path, and this code does not
									     know what is at it — either something of another type, or a read that
									     failed. Opening the editor would invite exactly the overwrite these
									     rows exist to prevent. -->
									<span class="whitespace-nowrap text-2xs text-hint">
										{b.occupiedBy ? 'Resolve in the workspace' : 'Check the workspace'}
									</span>
								{:else if b.reusedFrom && !b.stubKept}
									<!-- No action either: this row is done with, whether its own path was
									     deleted with the retarget or filled in afterwards. -->
									<span class="whitespace-nowrap text-2xs text-hint">Reused</span>
								{:else}
									<Button
										variant={b.done ? 'subtle' : 'accent'}
										unifiedSize="sm"
										disabled={working}
										onClick={() => startFilling(b)}
									>
										{b.done ? 'Saved' : canConnect ? 'Connect' : 'Fill in'}
									</Button>
								{/if}
							{/snippet}
						</ImportSetupRow>
					{/each}
				</ul>
			</div>
		{/if}

		<!-- Three different things to say, and which one depends on what is left. A missing
		     credential degrades the project; a missing data table ends it, because every app
		     queries tables that do not exist. Only the credential case is offered as
		     skippable — saying "you can skip this" above a missing data table would be
		     telling the user something that is not true. -->
		{#if outstanding === 0}
			<Alert type="success" title="You're all set" size="xs">
				Everything this project needs is configured. Finish, and it is ready to run.
			</Alert>
		{:else if pendingTables.length > 0}
			<Alert
				type="warning"
				title={missingTables.length > 0
					? 'The project will not run without this'
					: 'This could not be checked'}
				size="xs"
			>
				{#if missingTables.length > 0}
					The tables {missingTables.length === 1
						? 'this data table holds'
						: 'these data tables hold'}
					do not exist, and the project's apps and flows read them. Every one of those fails as soon
					as it opens.
				{/if}
				{#if uncheckedTables.length > 0}
					{#if missingTables.length > 0}<br /><br />{/if}
					{uncheckedTables.length === 1 ? 'One data table is' : 'Some data tables are'} set up, but
					{uncheckedTables.length === 1 ? 'its' : 'their'} schema could not be read, so whether the project's
					tables are there is unknown. Check again once the database is reachable.
				{/if}
			</Alert>
		{:else}
			<Alert type="info" title="You can skip this" size="xs" collapsible>
				The project's apps and flows will fail wherever they read a credential that is still
				missing. Everything else it imported works either way, and you can fill these in from the
				workspace at any time.
			</Alert>
		{/if}
	{/if}

	<div class="mt-2 flex items-center justify-between">
		{#if onBack}
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: ArrowLeft }}
				disabled={working}
				onClick={onBack}
			>
				Back
			</Button>
		{:else}
			<span></span>
		{/if}
		<div class="flex items-center gap-2">
			<!-- Every row carries its own action, so the footer only offers the way out —
			     twice, because leaving work undone is a different decision from having
			     finished it. Finish stays disabled until nothing is outstanding, and Skip
			     is the subtle escape beside it. A load that failed cannot tell what is
			     outstanding, so it offers Finish rather than blocking on an unknown — but a
			     load still *running* has the same empty lists as a step with nothing to do,
			     so Finish waits for it rather than reading that emptiness as "all done". -->
			{#if outstanding > 0 && !loading && !loadError}
				<Button variant="subtle" unifiedSize="sm" disabled={working} onClick={skip}>
					Skip for now
				</Button>
			{/if}
			<Button
				variant="accent"
				unifiedSize="sm"
				disabled={working || loading || (outstanding > 0 && !loadError)}
				onClick={onFinish}
			>
				Finish setup →
			</Button>
		</div>
	</div>
</div>

{#if wizardOpen || wizardFor}
	<AddDataTableWizard
		bind:this={wizard}
		bind:opened={wizardOpen}
		initialName={wizardFor}
		modalTarget="body"
		{workspace}
		finishAlso="run migrations"
		onFinishAlso={() => runMigrationsFor(retryTarget ?? '')}
		existingNames={configuredNames.map((c) => c.name)}
		existingDataTables={configuredNames}
		onDone={() => void afterWizard()}
		{customInstanceDbs}
		{confirmationModal}
		{defaultInstanceDbName}
	/>
{/if}
<!-- Portalled to the body, not left in place: this step renders inside the wizard page's
     CenteredModal, which is its own stacking context, while the data table wizard it shares
     this handle with portals to the body. In place, the confirmation's z-index is capped by
     that context and the wizard paints over it — leaving its backdrop swallowing every click
     with nothing visible to answer. -->
<Portal>
	<ConfirmationModal {...confirmationModal.props} />
</Portal>

<!-- The destination is not the workspace the app is in until the run switches to it,
     so the editor is told which one explicitly.

     Saving re-reads only the resources, never `load()`: a credential cannot change which
     data tables the project ships or which ones the workspace has, and `load()` raises
     `loading`, which replaces both lists with the spinner — so every save looked like the
     whole step had reloaded. -->
<ResourceEditorDrawer
	bind:this={resourceEditor}
	{workspace}
	onSaved={() => void refreshBlanks()}
	onRestored={() => void refreshBlanks()}
/>

<!-- `on:refresh` fires once the connection has been written into the stub — the same moment
     a save is — so the rows settle the same way either route was taken. -->
<AppConnectDrawer bind:this={appConnect} {workspace} on:refresh={() => void refreshBlanks()} />

<!-- What "Fill in" opens when the workspace already has a resource of the row's type.
     Portalled to the body for the reason the confirmation above is: this step renders inside
     the wizard page's CenteredModal, which is its own stacking context. -->
<Modal2
	title={choosing ? `Set up ${resourceTypeDisplayName(choosing.resourceType)}` : ''}
	target="body"
	fixedWidth="xs"
	fixedHeight="adaptive"
	formStyling
	closeOnOutsideClick={!reusing}
	bind:isOpen={
		() => choosing !== undefined,
		(v) => {
			if (!v && !reusing) choosing = undefined
		}
	}
>
	{#if choosing}
		{@const forRow = choosing}
		{@const existing = candidates[forRow.resourceType] ?? []}
		<div class="flex w-full flex-col gap-4 text-xs">
			<p class="text-secondary">
				This workspace already has {existing.length}
				{resourceTypeDisplayName(forRow.resourceType)}
				{existing.length === 1 ? 'resource' : 'resources'}. Use one and this project's apps, flows
				and triggers are pointed at it.
			</p>
			<Select
				bind:value={chosenPath}
				items={existing.map((p) => ({ value: p, label: p }))}
				placeholder="Pick a resource"
				disabled={reusing}
				clearable
				class="w-full"
			/>
			<div class="flex items-center justify-between gap-2">
				<Button variant="subtle" unifiedSize="sm" disabled={reusing} onClick={fillNewInstead}>
					{canConnectType(forRow.resourceType) ? 'Connect a new one' : 'Fill in a new one'}
				</Button>
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={!chosenPath || reusing}
					onClick={() => void reuseChosen()}
				>
					{reusing ? 'Pointing the project at it…' : 'Use this resource'}
				</Button>
			</div>
		</div>
	{/if}
</Modal2>

<script lang="ts">
	import { Button } from '../common'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Tabs from '../common/tabs/Tabs.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import TabContent from '../common/tabs/TabContent.svelte'
	import Toggle from '../Toggle.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import { WorkspaceService, type DatatableMigration } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { tick } from 'svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { fetchPendingMigrations, outOfOrderRunMessage } from './datatableMigrationUtils'
	import Select from '../select/Select.svelte'
	import { parseMigrationRole, withMigrationRole } from '../datatableMigrationRole'
	import { ADMIN_DATATABLE_ROLE } from '../dbTypes'
	import { resource } from 'runed'

	let {
		workspace,
		datatable,
		onCreated,
		onClose,
		onSeeMigration
	}: {
		workspace: string
		datatable: string
		/** Called after a successful create; `ran` is true when it was also run. */
		onCreated?: (ran: boolean) => void
		/** Called whenever the modal closes. `result` reports whether the close
		 * was due to a create (and whether that create was also run) vs a cancel —
		 * computed synchronously so callers don't depend on onCreated/onClose order. */
		onClose?: (result: { created: boolean; ran: boolean }) => void
		/** When set, the success toast gets a "See migration" action that calls this
		 * with the created migration so the caller can open it in the Migrations modal. */
		onSeeMigration?: (migration: DatatableMigration) => void
	} = $props()

	let isOpen = $state(false)
	// The reason the modal is about to close, set synchronously before `isOpen`
	// flips so the onClose effect reports it reliably regardless of effect timing.
	let closeResult = { created: false, ran: false }
	let prevOpen = false
	$effect(() => {
		if (prevOpen && !isOpen) {
			onClose?.(closeResult)
			closeResult = { created: false, ran: false }
		}
		prevOpen = isOpen
	})
	let tab = $state('up')
	let name = $state('')
	let nameInput = $state<TextInput>()
	let upEditor = $state<SimpleEditor | undefined>()
	let downEditor = $state<SimpleEditor | undefined>()
	// A valid migration name is non-empty and limited to letters, digits, '_' and '-'.
	const MIGRATION_NAME_RE = /^[a-zA-Z0-9_-]+$/
	let nameInvalid = $derived(!MIGRATION_NAME_RE.test(name.trim()))
	let codeUp = $state('')
	let enableDown = $state(false)
	let codeDown = $state('')
	let creating = $state(false)

	const confirmationModal = createAsyncConfirmationModal()

	// The role lives in the SQL as its `-- role <name>` annotation, so the code is the single
	// source of truth and the Select is a view onto it: reading parses, writing rewrites the
	// annotation.
	const usableRoles = resource(
		() => [workspace, datatable] as const,
		async ([ws, dt]) => {
			try {
				return await WorkspaceService.listUsableDatatableRoles({
					workspace: ws,
					datatableName: dt
				})
			} catch (e) {
				console.error('Failed to load data table roles:', e)
				return null
			}
		}
	)
	// Not a valid role name, so it cannot collide with one.
	const NO_ROLE = '(no role)'
	let declaredUp = $derived(parseMigrationRole(codeUp))
	let declaredDown = $derived(enableDown ? parseMigrationRole(codeDown) : undefined)
	let malformedLine = $derived(
		declaredUp.kind === 'malformed'
			? declaredUp.line
			: declaredDown?.kind === 'malformed'
				? declaredDown.line
				: undefined
	)
	const roleOf = (d: typeof declaredUp) => (d.kind === 'role' ? d.role : undefined)
	// A rollback runs as the role its own SQL names: under another role than the up migration it
	// typically cannot touch what the up created.
	let sqlProblem = $derived(
		malformedLine !== undefined
			? malformedMessage(malformedLine)
			: declaredDown !== undefined && roleOf(declaredDown) !== roleOf(declaredUp)
				? `The down migration runs as ${roleOf(declaredDown) ?? 'admin (no role)'} but the up migration as ${roleOf(declaredUp) ?? 'admin (no role)'}: make their role annotations match`
				: undefined
	)
	let selectedRole = $derived(declaredUp.kind === 'role' ? declaredUp.role : NO_ROLE)
	let permissioned = $derived(!!usableRoles.current?.permissioned)
	// No annotation runs as admin, which the server allows exactly to those who may use `admin`.
	let adminUsable = $derived(!!usableRoles.current?.roles.includes(ADMIN_DATATABLE_ROLE))
	let roleItems = $derived.by(() => {
		const usable = usableRoles.current
		if (!usable?.permissioned) return []
		const names = usable.roles.filter((r) => r !== ADMIN_DATATABLE_ROLE)
		// A role the SQL names but the caller cannot use is still shown, or the picker would
		// misreport what the migration runs as.
		if (declaredUp.kind === 'role' && !names.includes(declaredUp.role)) {
			names.push(declaredUp.role)
		}
		const items = names.map((r) => ({
			value: r,
			label: r === usable.default_role ? `${r} (default)` : r
		}))
		if (adminUsable || declaredUp.kind === 'none') {
			items.push({
				value: NO_ROLE,
				label: 'No role — runs as admin with full access'
			})
		}
		return items
	})

	function setRole(value: string | undefined) {
		const role = value === NO_ROLE ? undefined : value
		codeUp = withMigrationRole(codeUp, role)
		// Up and down agree: a rollback run as another role could fail on objects it does not own.
		if (enableDown) codeDown = withMigrationRole(codeDown, role)
		// Assigning the bound value does not repaint the editor, and its next keystroke would write
		// the stale text back.
		upEditor?.setCode(codeUp)
		if (enableDown) downEditor?.setCode(codeDown)
	}

	// Set by `open` when the SQL names no role yet: the data table's default is written once its
	// roles are known.
	let applyDefaultRole = $state(false)
	$effect(() => {
		// `undefined` until the first answer lands; `null` when it failed.
		const usable = usableRoles.current
		if (!applyDefaultRole || !isOpen || usableRoles.loading || usable === undefined) return
		applyDefaultRole = false
		if (!usable?.permissioned || declaredUp.kind !== 'none') return
		const role =
			usable.default_role !== ADMIN_DATATABLE_ROLE && usable.roles.includes(usable.default_role)
				? usable.default_role
				: usable.default_role === ADMIN_DATATABLE_ROLE && adminUsable
					? undefined
					: usable.roles.find((r) => r !== ADMIN_DATATABLE_ROLE)
		if (role !== undefined) setRole(role)
	})

	// Frame the migration body in an explicit transaction so it applies atomically.
	function wrapInTransaction(body: string): string {
		return `BEGIN;\n\n${body}\n\nEND;`
	}
	// Every statement inside BEGIN; ... END; must be `;`-terminated, so normalize
	// a prefilled body that ends without one.
	function ensureTrailingSemicolon(body: string): string {
		const trimmed = body.trimEnd()
		return trimmed.endsWith(';') ? trimmed : `${trimmed};`
	}
	const PLACEHOLDER = wrapInTransaction('-- Add your migration here')

	function malformedMessage(line: string): string {
		return `Malformed role annotation \`${line}\`: write it as \`-- role <name>\`, or pick the role above`
	}

	export function open(prefill?: { name?: string; codeUp?: string; codeDown?: string }) {
		// Roles and the default can have changed since the last open (the roles drawer sits next
		// to this modal), and the default role is written from this answer.
		usableRoles.refetch()
		name = prefill?.name ?? ''
		// Start from the transaction template; when prefilled from detected DDL,
		// wrap that DDL in the same BEGIN; ... END; frame. A role the prefill declares is taken
		// out first and put back on top: below `BEGIN;` it would not be read.
		const prefillRole = prefill?.codeUp ? parseMigrationRole(prefill.codeUp) : undefined
		if (prefill?.codeUp) {
			const wrapped = wrapInTransaction(
				ensureTrailingSemicolon(withMigrationRole(prefill.codeUp, undefined))
			)
			codeUp =
				prefillRole?.kind === 'role'
					? withMigrationRole(wrapped, prefillRole.role)
					: prefillRole?.kind === 'malformed'
						? `${prefillRole.line}\n${wrapped}`
						: wrapped
		} else {
			codeUp = PLACEHOLDER
		}
		codeDown = prefill?.codeDown ?? PLACEHOLDER
		enableDown = (prefill?.codeDown ?? '') !== ''
		applyDefaultRole = prefillRole === undefined || prefillRole.kind === 'none'
		tab = 'up'
		isOpen = true
		// Focus the name field once the modal content has rendered.
		tick().then(() => nameInput?.focus())
	}

	async function create(run: boolean) {
		if (name.trim() === '') {
			sendUserToast('Migration name is required', true)
			return
		}
		if (!MIGRATION_NAME_RE.test(name.trim())) {
			sendUserToast("Invalid migration name: use only letters, digits, '_' and '-'", true)
			return
		}
		if (sqlProblem !== undefined) {
			sendUserToast(sqlProblem, true)
			return
		}
		if (run) {
			// A new migration gets the highest timestamp, so any still-pending
			// migration is earlier: running only this one applies it out of order.
			// Warn just like the row-level Run action does. Best-effort: if the
			// status can't be fetched, fall through and let the run/rollback handle it.
			let pending: Awaited<ReturnType<typeof fetchPendingMigrations>> = []
			try {
				pending = await fetchPendingMigrations(workspace, datatable)
			} catch {}
			if (pending.length > 0) {
				const confirmed = await confirmationModal.ask({
					title: 'Run migration out of order',
					confirmationText: 'Run anyway',
					children: outOfOrderRunMessage(pending.length)
				})
				if (!confirmed) return
			}
		}
		creating = true
		try {
			const created = await WorkspaceService.createDatatableMigration({
				workspace,
				datatableName: datatable,
				requestBody: {
					name: name.trim(),
					code_up: codeUp,
					code_down: enableDown ? codeDown : undefined
				}
			})
			if (run) {
				try {
					await WorkspaceService.runDatatableMigrations({
						workspace,
						datatableName: datatable,
						only: created.timestamp
					})
				} catch (runErr: any) {
					// The migration was created but failed to run; undo the insertion so
					// the user can fix the SQL and retry from a clean state.
					await WorkspaceService.deleteDatatableMigration({
						workspace,
						datatableName: datatable,
						timestamp: created.timestamp
					}).catch(() => {})
					sendUserToast(
						`Migration failed to run and was reverted: ${runErr?.body ?? runErr?.message ?? runErr}`,
						true
					)
					return
				}
			}
			closeResult = { created: true, ran: run }
			onCreated?.(run)
			sendUserToast(
				run ? 'Migration created and run' : 'Migration created',
				'success',
				onSeeMigration
					? [{ label: 'See migration', callback: () => onSeeMigration?.(created) }]
					: []
			)
			isOpen = false
		} catch (e: any) {
			sendUserToast(`Failed to create migration: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			creating = false
		}
	}
</script>

<!-- closeOnOutsideClick is off: the "Create and run" split-button menu renders in a
	portal outside the modal, so an outside-click close would fire on its items and be
	mistaken for a cancel. Close via the header X or Escape instead. -->
<Modal2
	bind:isOpen
	title="New migration — {datatable}"
	fixedWidth="md"
	fixedHeight="adaptive"
	closeOnOutsideClick={false}
>
	<div class="flex flex-col gap-3 w-full grow min-h-0">
		<div class="flex gap-2 items-center">
			<TextInput
				bind:this={nameInput}
				bind:value={name}
				error={nameInvalid}
				class="grow"
				inputProps={{ placeholder: 'Migration name (e.g. add_index_to_customers)' }}
			/>
			{#if permissioned}
				<Select
					transformInputSelectedText={(s) => `Role: ${s}`}
					items={roleItems}
					bind:value={() => selectedRole, (r) => setRole(r)}
					placeholder="Role"
					class="w-72"
				/>
			{/if}
		</div>
		{#if sqlProblem !== undefined}
			<p class="text-xs text-red-500">{sqlProblem}</p>
		{:else if permissioned && usableRoles.current?.roles.length === 0}
			<p class="text-xs text-secondary">
				You can't use any role of this data table, so a migration you create can't be run.
			</p>
		{/if}
		<Tabs bind:selected={tab} class="grow min-h-0">
			<Tab value="up" label="Up" />
			<Tab value="down" label="Down" />
			{#snippet content()}
				<TabContent value="up" class="h-80 border rounded-md overflow-hidden">
					<SimpleEditor bind:this={upEditor} class="h-full" lang="sql" bind:code={codeUp} />
				</TabContent>
				<TabContent value="down" class="h-80">
					<div class="flex flex-col gap-2 h-full">
						<Toggle
							bind:checked={
								() => enableDown,
								(checked) => {
									enableDown = checked
									// The down editor is created by this toggle, so it reads the rewritten text.
									if (checked) {
										codeDown = withMigrationRole(
											codeDown,
											declaredUp.kind === 'role' ? declaredUp.role : undefined
										)
									}
								}
							}
							options={{ right: 'Enable down migration' }}
							size="sm"
						/>
						{#if enableDown}
							<div class="grow min-h-0 border rounded-md overflow-hidden">
								<SimpleEditor
									bind:this={downEditor}
									class="h-full"
									lang="sql"
									bind:code={codeDown}
								/>
							</div>
						{/if}
					</div>
				</TabContent>
			{/snippet}
		</Tabs>
		<div class="flex justify-end pt-2">
			<Button
				variant="accent"
				size="sm"
				disabled={creating || sqlProblem !== undefined}
				on:click={() => create(true)}
				dropdownItems={[
					{
						label: 'Create without running',
						onClick: () => create(false)
					}
				]}
			>
				Create and run
			</Button>
		</div>
	</div>
</Modal2>

<Portal>
	<ConfirmationModal {...confirmationModal.props} />
</Portal>

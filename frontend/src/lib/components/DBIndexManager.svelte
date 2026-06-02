<script lang="ts">
	import { ClipboardCopy, Key, Loader2, Plus, Trash2Icon, X } from 'lucide-svelte'
	import { resource } from 'runed'
	import { Button } from './common'
	import { Cell } from './table'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Select from './select/Select.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import Toggle from './Toggle.svelte'
	import Label from './Label.svelte'
	import Badge from './Badge.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import Portal from './Portal.svelte'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard } from '$lib/utils'
	import type { IDbIndexOps } from './dbOps'
	import {
		formatIndexSize,
		type CreateIndexInput,
		type DbIndex
	} from './apps/components/display/dbtable/queries/indexes'

	type Props = {
		dbIndexOps: IDbIndexOps
		tableKey: string
		schema?: string
		availableColumns: string[]
	}
	let { dbIndexOps, tableKey, schema, availableColumns }: Props = $props()

	const INDEX_METHODS = ['btree', 'hash', 'gin', 'gist', 'brin', 'spgist']

	let indexes = resource(
		() => [tableKey, schema] as const,
		async () => await dbIndexOps.listIndexes({ tableKey, schema })
	)

	function emptyForm(): CreateIndexInput {
		return {
			name: '',
			columns: [{ value: '', isExpression: false }],
			unique: false,
			method: 'btree',
			where: '',
			include: [],
			concurrent: true
		}
	}
	let form = $state<CreateIndexInput>(emptyForm())
	// MultiSelect requires a non-optional string[] binding, so INCLUDE columns
	// live in their own state and are merged into the payload on submit.
	let includeColumns = $state<string[]>([])
	let showAdvanced = $state(false)
	let busy = $state(false)

	type Confirm = {
		open: boolean
		title: string
		confirmationText: string
		code?: string
		loading?: boolean
		onConfirm: () => Promise<void>
	}
	let confirm = $state<Confirm | undefined>(undefined)

	let canCreate = $derived(form.columns.some((c) => c.value.trim().length > 0))

	function sanitizedForm(): CreateIndexInput {
		return {
			...form,
			columns: form.columns
				.filter((c) => c.value.trim().length > 0)
				.map((c) => ({ value: c.value.trim(), isExpression: c.isExpression })),
			include: includeColumns.filter((c) => c.length)
		}
	}

	function toastErr(e: unknown) {
		let msg: string | undefined = (e as Error)?.message
		if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : 'An error occurred'
		sendUserToast(msg, true)
	}

	async function openCreateConfirm() {
		busy = true
		try {
			const values = sanitizedForm()
			const sql = await dbIndexOps.previewCreateIndexSql({ tableKey, schema, values })
			confirm = {
				open: true,
				title: 'Create the following index?',
				confirmationText: 'Create index',
				code: sql,
				onConfirm: async () => {
					await dbIndexOps.createIndex({ tableKey, schema, values })
					sendUserToast('Index created')
					form = emptyForm()
					includeColumns = []
					showAdvanced = false
					indexes.refetch()
				}
			}
		} catch (e) {
			toastErr(e)
		} finally {
			busy = false
		}
	}

	async function openDropConfirm(idx: DbIndex) {
		busy = true
		try {
			const sql = await dbIndexOps.previewDropIndexSql({
				name: idx.name,
				schema,
				concurrent: true
			})
			confirm = {
				open: true,
				title: `Drop index "${idx.name}"?`,
				confirmationText: 'Drop index',
				code: sql,
				onConfirm: async () => {
					await dbIndexOps.dropIndex({ name: idx.name, schema, concurrent: true })
					sendUserToast('Index dropped')
					indexes.refetch()
				}
			}
		} catch (e) {
			toastErr(e)
		} finally {
			busy = false
		}
	}

	async function runConfirm() {
		if (!confirm) return
		confirm.loading = true
		try {
			await confirm.onConfirm()
			confirm = undefined
		} catch (e) {
			toastErr(e)
			if (confirm) confirm.loading = false
		}
	}
</script>

<div class="flex flex-col gap-6 p-2">
	<!-- Existing indexes -->
	<div class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<span class="font-semibold text-sm">Indexes</span>
			{#if indexes.loading}
				<Loader2 class="animate-spin" size={16} />
			{/if}
		</div>

		{#if indexes.error}
			<p class="text-red-500 text-sm">{indexes.error}</p>
		{:else if !indexes.current?.length && !indexes.loading}
			<p class="text-secondary text-sm">No indexes on this table.</p>
		{:else}
			<DataTable>
				<Head>
					<tr>
						<Cell head first>Name</Cell>
						<Cell head>Definition</Cell>
						<Cell head>Size</Cell>
						<Cell head last></Cell>
					</tr>
				</Head>
				<tbody class="divide-y bg-surface">
					{#each indexes.current ?? [] as idx (idx.name)}
						<tr>
							<Cell first>
								<div class="flex flex-col gap-1">
									<span class="font-mono text-xs break-all">{idx.name}</span>
									<div class="flex flex-wrap gap-1">
										{#if idx.isPrimary}
											<Badge twBgColor="bg-blue-100" twTextColor="text-blue-800">primary</Badge>
										{/if}
										{#if idx.isUnique && !idx.isPrimary}
											<Badge twBgColor="bg-green-100" twTextColor="text-green-800">unique</Badge>
										{/if}
										{#if idx.method}
											<Badge twBgColor="bg-surface-secondary" twTextColor="text-secondary">
												{idx.method}
											</Badge>
										{/if}
										{#if !idx.isValid}
											<Badge twBgColor="bg-red-100" twTextColor="text-red-800">invalid</Badge>
										{/if}
									</div>
								</div>
							</Cell>
							<Cell>
								<span class="font-mono text-xs whitespace-pre-wrap break-all text-secondary">
									{idx.definition}
								</span>
							</Cell>
							<Cell>
								<span class="text-xs text-secondary whitespace-nowrap">
									{formatIndexSize(idx.sizeBytes)}
								</span>
							</Cell>
							<Cell last>
								{#if idx.backsConstraint}
									<div
										class="flex items-center gap-1 text-tertiary text-xs"
										title="Backs a primary key or unique constraint — drop it via the constraint instead"
									>
										<Key size={14} />
									</div>
								{:else}
									<Button
										color="light"
										size="xs"
										iconOnly
										startIcon={{ icon: Trash2Icon }}
										on:click={() => openDropConfirm(idx)}
									/>
								{/if}
							</Cell>
						</tr>
					{/each}
				</tbody>
			</DataTable>
		{/if}
	</div>

	<!-- Create index -->
	<div class="flex flex-col gap-3 border-t pt-4">
		<span class="font-semibold text-sm">Create index</span>

		<Label label="Name (optional, auto-generated if empty)">
			<TextInput
				bind:value={form.name}
				inputProps={{ type: 'text', placeholder: 'idx_my_table_col' }}
			/>
		</Label>

		<Label label="Columns">
			<div class="flex flex-col gap-2">
				{#each form.columns as column, i (i)}
					<div class="flex items-center gap-2">
						<div class="grow">
							{#if column.isExpression}
								<TextInput
									bind:value={column.value}
									inputProps={{ type: 'text', placeholder: 'lower(email)' }}
								/>
							{:else}
								<Select
									items={availableColumns.map((c) => ({ value: c, label: c }))}
									bind:value={column.value}
									placeholder="Select column"
									clearable={false}
								/>
							{/if}
						</div>
						<Toggle
							size="xs"
							bind:checked={column.isExpression}
							options={{ right: 'expr', rightTooltip: 'Use a raw SQL expression' }}
						/>
						<Button
							color="light"
							size="xs"
							iconOnly
							startIcon={{ icon: X }}
							disabled={form.columns.length <= 1}
							on:click={() => form.columns.splice(i, 1)}
						/>
					</div>
				{/each}
				<Button
					color="light"
					size="xs"
					startIcon={{ icon: Plus }}
					on:click={() => form.columns.push({ value: '', isExpression: false })}
				>
					Add column
				</Button>
			</div>
		</Label>

		<div class="flex flex-wrap items-center gap-6">
			<Toggle
				bind:checked={form.unique}
				options={{ right: 'Unique', rightTooltip: 'Enforce uniqueness on the indexed columns' }}
			/>
			<Toggle
				bind:checked={form.concurrent}
				options={{
					right: 'Build concurrently',
					rightTooltip:
						'Build without locking the table against writes (recommended for production)'
				}}
			/>
			<div class="flex items-center gap-2">
				<span class="text-sm text-secondary">Method</span>
				<Select
					items={INDEX_METHODS.map((m) => ({ value: m, label: m }))}
					bind:value={form.method}
					clearable={false}
					class="!w-32"
				/>
			</div>
		</div>

		<Toggle
			size="xs"
			bind:checked={showAdvanced}
			options={{ right: 'Advanced (partial & covering)' }}
		/>
		{#if showAdvanced}
			<div class="flex flex-col gap-3 border-l-2 border-surface-selected pl-3">
				<Label label="Partial index predicate (WHERE)">
					<TextInput
						bind:value={form.where}
						inputProps={{ type: 'text', placeholder: 'active = true' }}
					/>
				</Label>
				<Label label="Include columns (covering)">
					<MultiSelect
						items={availableColumns.map((c) => ({ value: c, label: c }))}
						bind:value={includeColumns}
						placeholder="Select columns to INCLUDE"
					/>
				</Label>
			</div>
		{/if}

		<div>
			<Button disabled={!canCreate} loading={busy} on:click={openCreateConfirm}>
				Create index
			</Button>
		</div>
	</div>
</div>

<Portal>
	<ConfirmationModal
		id="db-index-manager-confirmation-modal"
		title={confirm?.title ?? ''}
		confirmationText={confirm?.confirmationText ?? ''}
		open={confirm?.open ?? false}
		loading={confirm?.loading ?? false}
		onConfirmed={runConfirm}
		onCanceled={() => (confirm = undefined)}
	>
		{#if confirm?.code}
			<div
				class="bg-surface-secondary border border-surface-selected rounded-md p-2 relative group"
			>
				<button
					class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-surface-hover"
					onclick={() => copyToClipboard(confirm?.code)}
					title="Copy to clipboard"
				>
					<ClipboardCopy size={14} />
				</button>
				<pre class="whitespace-pre-wrap text-sm"><code>{confirm.code}</code></pre>
			</div>
		{/if}
	</ConfirmationModal>
</Portal>

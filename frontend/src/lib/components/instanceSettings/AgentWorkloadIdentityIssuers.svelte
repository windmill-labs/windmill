<script lang="ts">
	import type { Writable } from 'svelte/store'
	import { Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { Plus, Trash2, X } from 'lucide-svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const KEY = 'agent_workload_identity_issuers'

	type KeySource = 'discovery' | 'jwks_uri' | 'inline'
	type ClaimRow = { id: number; path: string; value: string }
	type RuleRow = { id: number; claims: ClaimRow[]; worker_group: string; tags: string }
	type IssuerRow = {
		id: number
		issuer: string
		audience: string
		keySource: KeySource
		jwks_uri: string
		jwks: string
		rules: RuleRow[]
	}

	let nextId = 0
	// Rows keep a stable identity while editing; `lastSynced` tells our own writes to
	// `$values[KEY]` apart from external ones (load, save, discard), which re-hydrate.
	let issuers = $state<IssuerRow[]>([])
	let lastSynced: string | undefined = undefined

	function newRule(): RuleRow {
		return {
			id: nextId++,
			claims: [{ id: nextId++, path: 'sub', value: '' }],
			worker_group: '',
			tags: ''
		}
	}

	function toRows(v: unknown): IssuerRow[] {
		if (!Array.isArray(v)) return []
		return v.map((i: any) => ({
			id: nextId++,
			issuer: i.issuer ?? '',
			audience: i.audience ?? '',
			keySource: i.jwks ? 'inline' : i.jwks_uri ? 'jwks_uri' : 'discovery',
			jwks_uri: i.jwks_uri ?? '',
			jwks: i.jwks ?? '',
			rules: (i.rules ?? []).map((r: any) => ({
				id: nextId++,
				claims: Object.entries(r.claims ?? {}).map(([path, value]) => ({
					id: nextId++,
					path,
					value: String(value)
				})),
				worker_group: r.worker_group ?? '',
				tags: (r.tags ?? []).join(', ')
			}))
		}))
	}

	function serialize(rows: IssuerRow[]): any[] | undefined {
		if (rows.length === 0) return undefined
		return rows.map((i) => ({
			issuer: i.issuer.trim(),
			...(i.keySource === 'jwks_uri' && i.jwks_uri.trim() ? { jwks_uri: i.jwks_uri.trim() } : {}),
			...(i.keySource === 'inline' && i.jwks.trim() ? { jwks: i.jwks.trim() } : {}),
			audience: i.audience.trim(),
			rules: i.rules.map((r) => ({
				claims: Object.fromEntries(
					r.claims.filter((c) => c.path.trim()).map((c) => [c.path.trim(), c.value])
				),
				worker_group: r.worker_group.trim(),
				tags: r.tags
					.split(',')
					.map((t) => t.trim())
					.filter((t) => t)
			}))
		}))
	}

	$effect(() => {
		const cur = JSON.stringify($values[KEY] ?? null)
		if (cur === lastSynced) return
		lastSynced = cur
		issuers = toRows($values[KEY])
	})

	$effect(() => {
		const next = serialize(issuers)
		if (lastSynced === undefined) return
		const nextStr = JSON.stringify(next ?? null)
		// Compare against the stored value normalized the same way, so loading a value whose
		// keys are ordered differently does not mark the form dirty.
		if (nextStr === JSON.stringify(serialize(toRows($values[KEY])) ?? null)) return
		lastSynced = nextStr
		$values[KEY] = next
	})

	function addIssuer() {
		issuers.push({
			id: nextId++,
			issuer: '',
			audience: 'windmill',
			keySource: 'discovery',
			jwks_uri: '',
			jwks: '',
			rules: [newRule()]
		})
	}
</script>

<div class="flex flex-col gap-4">
	{#each issuers as issuer, i (issuer.id)}
		<div class="flex flex-col gap-3 p-4 rounded-md border">
			<div class="flex items-center justify-between">
				<span class="text-xs font-semibold text-emphasis">Issuer {i + 1}</span>
				<Button
					variant="subtle"
					unifiedSize="xs"
					startIcon={{ icon: Trash2 }}
					iconOnly
					{disabled}
					aria-label="Remove issuer"
					onclick={() => issuers.splice(i, 1)}
				/>
			</div>
			<label class="flex flex-col gap-1">
				<span class="text-xs text-secondary">Issuer (must equal the token's iss)</span>
				<TextInput
					bind:value={issuer.issuer}
					inputProps={{ placeholder: 'https://kubernetes.default.svc.cluster.local', disabled }}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-xs text-secondary">Audience (must be in the token's aud)</span>
				<TextInput
					bind:value={issuer.audience}
					inputProps={{ placeholder: 'windmill', disabled }}
				/>
			</label>
			<div class="flex flex-col gap-1">
				<span class="text-xs text-secondary">Signing keys</span>
				<ToggleButtonGroup bind:selected={issuer.keySource} {disabled}>
					{#snippet children({ item })}
						<ToggleButton
							value="discovery"
							label="OIDC discovery"
							tooltip="Read jwks_uri from <issuer>/.well-known/openid-configuration"
							{item}
						/>
						<ToggleButton value="jwks_uri" label="JWKS URL" {item} />
						<ToggleButton
							value="inline"
							label="Inline JWKS"
							tooltip="For an issuer this instance cannot reach, e.g. the output of kubectl get --raw /openid/v1/jwks"
							{item}
						/>
					{/snippet}
				</ToggleButtonGroup>
				{#if issuer.keySource === 'jwks_uri'}
					<TextInput
						bind:value={issuer.jwks_uri}
						inputProps={{ placeholder: 'https://issuer.example.com/keys', disabled }}
					/>
				{:else if issuer.keySource === 'inline'}
					<TextInput
						underlyingInputEl="textarea"
						bind:value={issuer.jwks}
						inputProps={{ placeholder: '{"keys": [...]}', rows: 4, disabled }}
					/>
				{/if}
			</div>
			<div class="flex flex-col gap-2">
				<span class="text-xs text-secondary">
					Rules, tried in order: the first whose claims all match gives the agent its worker group
					and tags. A token no rule matches is refused.
				</span>
				{#each issuer.rules as rule, j (rule.id)}
					<div class="flex flex-col gap-2 border-l-2 border-surface-selected pl-3">
						<div class="flex items-center justify-between">
							<span class="text-2xs font-semibold text-secondary">Rule {j + 1}</span>
							<Button
								variant="subtle"
								unifiedSize="xs"
								startIcon={{ icon: Trash2 }}
								iconOnly
								{disabled}
								aria-label="Remove rule"
								onclick={() => issuer.rules.splice(j, 1)}
							/>
						</div>
						{#each rule.claims as claim, k (claim.id)}
							<div class="flex items-center gap-2">
								<TextInput
									bind:value={claim.path}
									class="w-64"
									inputProps={{ placeholder: 'kubernetes.io.namespace', disabled }}
								/>
								<span class="text-xs text-secondary">=</span>
								<TextInput
									bind:value={claim.value}
									inputProps={{ placeholder: 'system:serviceaccount:<ns>:<sa>', disabled }}
								/>
								<Button
									variant="subtle"
									unifiedSize="xs"
									startIcon={{ icon: X }}
									iconOnly
									disabled={disabled || rule.claims.length <= 1}
									aria-label="Remove claim"
									onclick={() => rule.claims.splice(k, 1)}
								/>
							</div>
						{/each}
						<div class="flex">
							<Button
								variant="subtle"
								unifiedSize="xs"
								startIcon={{ icon: Plus }}
								{disabled}
								onclick={() => rule.claims.push({ id: nextId++, path: '', value: '' })}
							>
								Add claim
							</Button>
						</div>
						<div class="flex gap-2">
							<label class="flex flex-col gap-1 w-48">
								<span class="text-2xs text-secondary">Worker group</span>
								<TextInput
									bind:value={rule.worker_group}
									inputProps={{ placeholder: 'k8s', disabled }}
								/>
							</label>
							<label class="flex flex-col gap-1 grow">
								<span class="text-2xs text-secondary">Tags (comma separated)</span>
								<TextInput
									bind:value={rule.tags}
									inputProps={{ placeholder: 'dbt, etl', disabled }}
								/>
							</label>
						</div>
					</div>
				{/each}
				<div class="flex">
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: Plus }}
						{disabled}
						onclick={() => issuer.rules.push(newRule())}
					>
						Add rule
					</Button>
				</div>
			</div>
		</div>
	{/each}
	<div class="flex">
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: Plus }}
			{disabled}
			onclick={addIssuer}
		>
			Add issuer
		</Button>
	</div>
</div>

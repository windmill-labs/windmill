<script lang="ts">
	import Tabs from '../common/tabs/Tabs.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import TabContent from '../common/tabs/TabContent.svelte'
	import HighlightCode from '../HighlightCode.svelte'

	let { name }: { name: string } = $props()

	let tab = $state<'python3' | 'duckdb' | 'bun'>('python3')

	// The SDKs default to `main`, so naming it would teach an argument nobody needs to write.
	let sdkArg = $derived(name === 'main' ? '' : `'${name}'`)

	let snippets = $derived({
		python3: `import wmill

def main():
    db = wmill.datatable(${sdkArg})
    return db.query("SELECT * FROM my_table LIMIT 10").fetch()`,
		duckdb: `ATTACH 'datatable://${name}' AS db;

SELECT * FROM db.my_table LIMIT 10;`,
		bun: `import * as wmill from 'windmill-client'

export async function main() {
  const sql = wmill.datatable(${sdkArg})
  return await sql\`SELECT * FROM my_table LIMIT 10\`.fetch()
}`
	})
</script>

<Tabs bind:selected={tab}>
	<Tab value="python3" label="Python" />
	<Tab value="duckdb" label="DuckDB" />
	<Tab value="bun" label="TypeScript" />
	{#snippet content()}
		{#each ['python3', 'duckdb', 'bun'] as const as lang (lang)}
			<TabContent value={lang} class="pt-2">
				<HighlightCode language={lang} code={snippets[lang]} className="text-xs" />
			</TabContent>
		{/each}
	{/snippet}
</Tabs>

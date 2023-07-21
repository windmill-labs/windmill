<script lang="ts">
	import { JobService, Preview } from '$lib/gen'
	import { dbSchema, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy } from 'svelte'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'

	export let resourceType: string | undefined
	export let pg: String | undefined = undefined

	let drawer: Drawer

	async function getSchema() {
		const content = `
import { Client } from "https://deno.land/x/postgres/mod.ts";

export async function main(pg: Postgresql) {
  // Create a new client with the provided connection details
  const u = new URL("postgres://")
	u.hash = ''
	u.search = '?sslmode=' + pg.sslmode
	u.pathname = pg.dbname
	u.host = pg.host
	u.port = pg.port
	u.password = pg.password
	u.username = pg.user

  const client = new Client(u.toString())

  // Connect to the postgres database
  await client.connect();

  const result = await client.queryObject(\`SELECT 
      table_name, 
      column_name, 
      udt_name,
      column_default,
      is_nullable,
      table_schema
    FROM 
      information_schema.columns
    WHERE table_schema != 'pg_catalog' AND 
      table_schema != 'information_schema'\`);

  const schemas = result.rows.reduce((acc, a) => {
    const table_schema = a.table_schema;
    delete a.table_schema;
    acc[table_schema] = acc[table_schema] || [];
    acc[table_schema].push(a);
    return acc;
  }, {});

  const data = {};
  for (const key in schemas) {
    data[key] = schemas[key].reduce((acc, a) => {
      const table_name = a.table_name;
      delete a.table_name;
      acc[table_name] = acc[table_name] || {};
      const p = {
        type: a.udt_name,
        required: a.is_nullable === "NO",
      }
      if (a.column_default) {
        p.default = a.column_default
      }
      acc[table_name][a.column_name] = p;
      return acc;
    }, {});
  }

  return data;
}`
		try {
			const job = await JobService.runScriptPreview({
				workspace: $workspaceStore!,
				requestBody: {
					language: 'deno' as Preview.language,
					content,
					args: {
						pg: '$res:' + pg
					}
				}
			})
			await new Promise((r) => setTimeout(r, 3000))
			const testResult = await JobService.getCompletedJob({
				workspace: $workspaceStore!,
				id: job
			})
			if (testResult) {
				if (!testResult.success) {
					throw new Error('Could not query DB schema')
				} else {
					dbSchema.set(testResult.result)
				}
			}
		} catch (err) {
			console.error(err)
			sendUserToast('Could not query DB schema', true)
		}
	}

	$: pg && resourceType === 'postgresql' && getSchema()
	$: !pg && $dbSchema && dbSchema.set(undefined)

	onDestroy(() => {
		dbSchema.set(undefined)
	})
</script>

{#if $dbSchema && pg}
	<Button
		size="xs"
		variant="contained"
		color="light"
		btnClasses="text-frost-500 !hover:text-frost-700"
		spacingSize="xs2"
		on:click={drawer.openDrawer}
	>
		Explore DB schema
	</Button>
	<Drawer bind:this={drawer} size="800px">
		<DrawerContent title="DB Schema Explorer" on:close={drawer.closeDrawer}>
			<ObjectViewer json={$dbSchema} pureViewer />
		</DrawerContent>
	</Drawer>
{/if}

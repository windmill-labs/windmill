<script lang="ts">
	import { JobService, Preview } from '$lib/gen'
	import { dbSchema, workspaceStore } from '$lib/stores'
	import { onDestroy } from 'svelte'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import { tryEvery } from '$lib/utils'

	export let resourceType: string | undefined
	export let resourcePath: String | undefined = undefined

	let drawer: Drawer

	const content = {
		postgresql: `import { Client } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
export async function main(args: any) {
  // Create a new client with the provided connection details
  const u = new URL("postgres://")
	u.hash = ''
	u.search = '?sslmode=' + args.sslmode
	u.pathname = args.dbname
	u.host = args.host
	u.port = args.port
	u.password = args.password
	u.username = args.user
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
}`,
		mysql: `import { Client } from "https://deno.land/x/mysql@v2.11.0/mod.ts";
export async function main(args: any) {
  const conn = await new Client().connect({
    hostname: args.host,
    port: args.port,
    username: args.user,
    db: args.database,
    password: args.password,
  });
  const result = await conn.execute(
    "select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT from information_schema.columns where table_schema != 'information_schema'",
  );
  const schemas = result.rows.reduce((acc, a) => {
    const table_schema = a.TABLE_SCHEMA;
    delete a.TABLE_SCHEMA;
    acc[table_schema] = acc[table_schema] || [];
    acc[table_schema].push(a);
    return acc;
  }, {});
  const data = {};
  for (const key in schemas) {
    data[key] = schemas[key].reduce((acc, a) => {
      const table_name = a.TABLE_NAME;
      delete a.TABLE_NAME;
      acc[table_name] = acc[table_name] || {};
      const p = {
        type: a.DATA_TYPE,
        required: a.is_nullable === "NO",
      };
      if (a.column_default) {
        p.default = a.COLUMN_DEFAULT;
      }
      acc[table_name][a.COLUMN_NAME] = p;
      return acc;
    }, {});
  }
  return data;
}`
	}

	async function getSchema() {
		if (!resourceType || !resourcePath) return
		dbSchema.set(undefined)

		const job = await JobService.runScriptPreview({
			workspace: $workspaceStore!,
			requestBody: {
				language: 'deno' as Preview.language,
				content: content[resourceType],
				args: {
					args: '$res:' + resourcePath
				}
			}
		})

		tryEvery({
			tryCode: async () => {
				const testResult = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
					id: job
				})
				if (!testResult.success) {
					console.error(testResult.result?.['error']?.['message'])
				} else {
					dbSchema.set(testResult.result)
				}
			},
			timeoutCode: async () => {
				console.error('Could not query DB schema within 5s')
				try {
					await JobService.cancelQueuedJob({
						workspace: $workspaceStore!,
						id: job,
						requestBody: {
							reason: 'Could not query DB schema within 5s'
						}
					})
				} catch (err) {
					console.error(err)
				}
			},
			interval: 500,
			timeout: 5000
		})
	}

	$: resourcePath && resourceType && ['postgresql', 'mysql'].includes(resourceType) && getSchema()
	$: !resourcePath && $dbSchema && dbSchema.set(undefined)

	onDestroy(() => {
		dbSchema.set(undefined)
	})
</script>

{#if $dbSchema && resourcePath}
	<Button
		size="xs"
		variant="border"
		color="blue"
		spacingSize="xs2"
		btnClasses="mt-1"
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

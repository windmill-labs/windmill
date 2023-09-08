<script lang="ts">
	import { JobService, Preview } from '$lib/gen'
	import {
		dbSchemas,
		workspaceStore,
		type DBSchema,
		type GraphqlSchema,
		type SQLSchema
	} from '$lib/stores'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import { sendUserToast, tryEvery } from '$lib/utils'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { buildClientSchema, printSchema } from 'graphql'
	import GraphqlSchemaViewer from './GraphqlSchemaViewer.svelte'
	import { faRefresh } from '@fortawesome/free-solid-svg-icons'

	export let resourceType: string | undefined
	export let resourcePath: string | undefined = undefined
	let dbSchema: DBSchema | undefined = undefined
	let loading = false

	let drawer: Drawer | undefined

	const scripts: {
		[key: string]: {
			code: string
			lang: string
		}
	} = {
		postgresql: {
			lang: 'deno',
			code: `import { Client } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
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
}`
		},
		mysql: {
			code: `import { Client } from "https://deno.land/x/mysql@v2.11.0/mod.ts";
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
}`,
			lang: 'deno'
		},
		graphql: {
			code: `import { getIntrospectionQuery } from "npm:graphql@16.7.1";
export async function main(args: any) {
  const headers: { [key: string]: string } = {
    "Content-Type": "application/json",
  };
  if (args.bearer_token) {
    headers["authorization"] = "Bearer " + args.bearer_token;
  }
  const response = await fetch(args.base_url, {
    method: "POST",
    headers,
    body: JSON.stringify({
      query: getIntrospectionQuery(),
    }),
  });
  if (!response.ok) {
    throw new Error("Could not query schema");
  }
  const schema = (await response.json()).data;
  return schema;
}`,
			lang: 'deno'
		},
		bigquery: {
			code: `
#requirements:
#google-cloud-bigquery==3.11.4
from google.cloud import bigquery as bq
from google.oauth2 import service_account
def main(args):
    credentials = service_account.Credentials.from_service_account_info(args)
    client = bq.Client(credentials=credentials)
    datasets = list(client.list_datasets())  # Make an API request.
    schema = dict()
    for dataset in datasets:
        schema[dataset.dataset_id] = dict()
        query = f"""
SELECT 
  table_name, ARRAY_AGG(STRUCT( 
      if(is_nullable = 'YES', true, false) AS required,
      column_name AS name,
      data_type AS type,
      if(column_default = 'NULL', null, column_default) AS \`default\`)
    ORDER BY ordinal_position) AS schema
FROM
  \`{dataset.dataset_id}\`.INFORMATION_SCHEMA.COLUMNS
GROUP BY
  table_name
"""
        query_job = client.query(query)  # API request
        rows = query_job.result()
        for row in rows:
            cols = []
            for col in row[1]:
                if col['default'] is None:
                    del col['default']
                cols.append(col)
            schema[dataset.dataset_id][row[0]] = cols
    return schema
`,
			lang: 'python3'
		},
		snowflake: {
			code: `# requirements:
# snowflake-connector-python==3.2.0
from typing import Any
import snowflake.connector as sf
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import serialization
def main(args):
    if not args["database"]:
        raise Exception("a selected database is required for the schema explorer")
    p_key = serialization.load_pem_private_key(
        args["private_key"].encode(), password=None, backend=default_backend()
    )
    pkb = p_key.private_bytes(
        encoding=serialization.Encoding.DER,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption(),
    )
    ctx = sf.connect(
        user=args["username"],
        account=args["account_identifier"],
        private_key=pkb,
        warehouse=args["warehouse"],
        database=args["database"],
        schema=args["schema"],
        role=args["role"],
    )
    cs = ctx.cursor()
    rows = cs.execute("select TABLE_SCHEMA, TABLE_NAME, DATA_TYPE, COLUMN_NAME, COLUMN_DEFAULT, IS_NULLABLE from information_schema.columns where table_schema != 'INFORMATION_SCHEMA'")
    schema = dict()
    for row in rows:
        if row[0] not in schema:
            schema[row[0]] = dict()
        if row[1] not in schema[row[0]]:
            schema[row[0]][row[1]] = dict()
        schema[row[0]][row[1]][row[3]] = {
            "type": row[2],
            "required": row[5] == "YES",
        }
        if row[4] is not None:
            schema[row[0]][row[1]][row[3]]["default"] = row[4]
    return schema
`,
			lang: 'python3'
		}
	}

	async function getSchema() {
		if (!resourceType || !resourcePath) return
		loading = true

		const job = await JobService.runScriptPreview({
			workspace: $workspaceStore!,
			requestBody: {
				language: scripts[resourceType].lang as Preview.language,
				content: scripts[resourceType].code,
				args: {
					args: '$res:' + resourcePath
				}
			}
		})

		tryEvery({
			tryCode: async () => {
				if (resourcePath) {
					const testResult = await JobService.getCompletedJob({
						workspace: $workspaceStore!,
						id: job
					})
					if (!testResult.success) {
						console.error(testResult.result?.['error']?.['message'])
					} else {
						if (resourceType !== undefined) {
							if (resourceType !== 'graphql') {
								$dbSchemas[resourcePath] = {
									lang: resourceType as SQLSchema['lang'],
									schema: testResult.result,
									publicOnly: !!testResult.result.public || !!testResult.result.PUBLIC
								}
							} else {
								$dbSchemas[resourcePath] = {
									lang: 'graphql',
									schema: testResult.result
								}
							}
						}
					}
				}
				loading = false
			},
			timeoutCode: async () => {
				loading = false
				console.error('Could not query schema within 5s')
				if (drawer?.isOpen()) {
					sendUserToast('Could not query schema within 5s', true)
				}
				try {
					await JobService.cancelQueuedJob({
						workspace: $workspaceStore!,
						id: job,
						requestBody: {
							reason: 'Could not query schema within 5s'
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

	function formatSchema(dbSchema: DBSchema) {
		if (dbSchema.lang !== 'graphql' && dbSchema.publicOnly) {
			return dbSchema.schema.public || dbSchema.schema.PUBLIC || dbSchema
		} else if (dbSchema.lang === 'mysql' && Object.keys(dbSchema.schema).length === 1) {
			return dbSchema.schema[Object.keys(dbSchema.schema)[0]]
		} else {
			return dbSchema.schema
		}
	}

	function formatGraphqlSchema(dbSchema: GraphqlSchema) {
		return printSchema(buildClientSchema(dbSchema.schema))
	}

	$: resourcePath &&
		Object.keys(scripts).includes(resourceType || '') &&
		!$dbSchemas[resourcePath] &&
		getSchema()

	$: dbSchema = resourcePath && resourcePath in $dbSchemas ? $dbSchemas[resourcePath] : undefined
</script>

{#if dbSchema}
	<Button
		size="xs"
		variant="border"
		color="blue"
		spacingSize="xs2"
		btnClasses="mt-1"
		on:click={drawer?.openDrawer}
	>
		Explore schema
	</Button>
	<Drawer bind:this={drawer}>
		<DrawerContent title="Schema Explorer" on:close={drawer.closeDrawer}>
			<svelte:fragment slot="actions">
				<Button
					on:click={getSchema}
					startIcon={{
						icon: faRefresh
					}}
					{loading}
					size="xs"
					color="light"
					>Refresh
				</Button>
			</svelte:fragment>
			{#if dbSchema.lang !== 'graphql' && (dbSchema.schema?.public || dbSchema.schema?.PUBLIC)}
				<ToggleButtonGroup class="mb-4" bind:selected={dbSchema.publicOnly}>
					<ToggleButton value={true} label="Public" />
					<ToggleButton value={false} label="All" />
				</ToggleButtonGroup>
			{/if}
			{#if dbSchema.lang === 'graphql'}
				<GraphqlSchemaViewer code={formatGraphqlSchema(dbSchema)} class="h-full" />
			{:else}
				<ObjectViewer json={formatSchema(dbSchema)} pureViewer />
			{/if}
		</DrawerContent>
	</Drawer>
{/if}

import { JobService, ResourceService } from "$lib/gen";

import { runScriptAndPollResult } from "$lib/components/jobs/utils";
import type { DbInput } from "$lib/components/dbTypes";
import {
	type ForeignKeyMetadata,
	getLanguageByResourceType,
	resourceTypeToLang,
	scripts,
	type TableMetadata,
} from "./utils";

import { type Preview } from "$lib/gen";
import type {
	DBSchema,
	DBSchemas,
	GraphqlSchema,
	SQLSchema,
} from "$lib/stores";

import { tryEvery } from "$lib/utils";
import { stringifySchema } from "$lib/components/copilot/lib";
import type { DbType } from "$lib/components/dbTypes";
import { getDatabaseArg } from "$lib/components/dbOps";

export async function loadTableMetaData(
	input: DbInput,
	workspace: string | undefined,
	table: string | undefined,
): Promise<TableMetadata | undefined> {
	if (!input || !table || !workspace) return undefined;

	let language = input.type == "ducklake"
		? "duckdb"
		: getLanguageByResourceType(input.resourceType);
	let content = await makeLoadTableMetaDataQuery(input, workspace, table);

	const job = await JobService.runScriptPreview({
		workspace: workspace,
		requestBody: {
			language,
			content,
			args: getDatabaseArg(input),
		},
	});

	const maxRetries = 8;
	let attempts = 0;
	while (attempts < maxRetries) {
		try {
			await new Promise((resolve) =>
				setTimeout(resolve, 1000 * (attempts || 0.6))
			);

			const testResult = (await JobService.getCompletedJob({
				workspace: workspace,
				id: job,
			})) as any;

			if (testResult.success) {
				attempts = maxRetries;

				if (
					input.type === "database" && input.resourceType === "ms_sql_server"
				) {
					return testResult.result[0].map(lowercaseKeys);
				} else {
					return testResult.result.map(lowercaseKeys);
				}
			} else {
				attempts++;
			}
		} catch (error) {
			attempts++;
		}
	}

	console.error("Failed to load table metadata after maximum retries.");
	return undefined;
}

export async function loadAllTablesMetaData(
	workspace: string | undefined,
	input: DbInput,
): Promise<Record<string, TableMetadata> | undefined> {
	if (!input || !workspace) return undefined;

	let language = input.type == "ducklake"
		? "duckdb"
		: getLanguageByResourceType(input.resourceType);

	try {
		let result = (await runScriptAndPollResult({
			workspace: workspace,
			requestBody: {
				language,
				content: await makeLoadTableMetaDataQuery(input, workspace, undefined),
				args: getDatabaseArg(input),
			},
		})) as ({ table_name: string; schema_name?: string } & object)[];
		if (input.type === "database" && input.resourceType === "ms_sql_server") {
			result = (result as any)[0];
		}
		const map: Record<string, TableMetadata> = {};

		for (const _col of result) {
			const col = lowercaseKeys(_col);
			const tableKey = col.schema_name
				? `${col.schema_name}.${col.table_name}`
				: col.table_name;
			if (!(tableKey in map)) {
				map[tableKey] = [];
			}
			map[tableKey].push(col);
		}
		return map;
	} catch (e) {
		throw new Error("Error loading all tables metadata: " + e);
	}
}

async function makeLoadTableMetaDataQuery(
	input: DbInput,
	workspace: string,
	table: string | undefined,
): Promise<string> {
	if (input.type === "ducklake") {
		return `ATTACH 'ducklake://${input.ducklake}' AS __ducklake__;
		SELECT
			COLUMN_NAME as field,
			DATA_TYPE as DataType,
			COLUMN_DEFAULT as DefaultValue,
			false as IsPrimaryKey,
			false as IsIdentity,
			IS_NULLABLE as IsNullable,
			false as IsEnum,
			TABLE_NAME as table_name
		FROM information_schema.columns c
		WHERE table_catalog = '__ducklake__' AND table_schema = current_schema()`;
	} else if (input.resourceType === "mysql") {
		const resourceObj = (await ResourceService.getResourceValue({
			workspace,
			path: input.resourcePath,
		})) as any;
		return `
		SELECT
			COLUMN_NAME as field,
			COLUMN_TYPE as DataType,
			COLUMN_DEFAULT as DefaultValue,
			CASE WHEN COLUMN_KEY = 'PRI' THEN 'YES' ELSE 'NO' END as IsPrimaryKey,
			CASE WHEN EXTRA like '%auto_increment%' THEN 'YES' ELSE 'NO' END as IsIdentity,
			CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,
			CASE WHEN DATA_TYPE = 'enum' THEN true ELSE false END as IsEnum${
			table ? "" : `,
			TABLE_NAME as table_name`
		}
	FROM
			INFORMATION_SCHEMA.COLUMNS${
			table
				? `
	WHERE
			TABLE_NAME = '${table.split(".").reverse()[0]}' AND TABLE_SCHEMA = '${
					table.split(".").reverse()[1] ?? resourceObj?.database ?? ""
				}'`
				: `
	WHERE
			TABLE_SCHEMA NOT IN ('mysql', 'performance_schema', 'information_schema', 'sys')`
		}
	ORDER BY
			TABLE_NAME,
			ORDINAL_POSITION;
	`;
	} else if (input.resourceType === "postgresql") {
		return `
	SELECT
		a.attname as field,
		pg_catalog.format_type(a.atttypid, a.atttypmod) as DataType,
		(SELECT substring(pg_catalog.pg_get_expr(d.adbin, d.adrelid, true) for 128)
		 FROM pg_catalog.pg_attrdef d
		 WHERE d.adrelid = a.attrelid AND d.adnum = a.attnum AND a.atthasdef) as DefaultValue,
		(SELECT CASE WHEN i.indisprimary THEN true ELSE 'NO' END
		 FROM pg_catalog.pg_class tbl, pg_catalog.pg_class idx, pg_catalog.pg_index i, pg_catalog.pg_attribute att
		 WHERE tbl.oid = a.attrelid AND idx.oid = i.indexrelid AND att.attrelid = tbl.oid
								 AND i.indrelid = tbl.oid AND att.attnum = any(i.indkey) AND att.attname = a.attname LIMIT 1) as IsPrimaryKey,
		CASE a.attidentity
				WHEN 'd' THEN 'By Default'
				WHEN 'a' THEN 'Always'
				ELSE 'No'
		END as IsIdentity,
		CASE a.attnotnull
				WHEN false THEN 'YES'
				ELSE 'NO'
		END as IsNullable,
		(SELECT true
		 FROM pg_catalog.pg_enum e
		 WHERE e.enumtypid = a.atttypid FETCH FIRST ROW ONLY) as IsEnum${
			table ? "" : `,
    ns.nspname AS schema_name,
    c.relname AS table_name`
		}
	FROM pg_catalog.pg_attribute a${
			table
				? `
	WHERE a.attrelid = (SELECT c.oid FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace ns ON c.relnamespace = ns.oid WHERE relname = '${
					table.split(".").reverse()[0]
				}' AND ns.nspname = '${table.split(".").reverse()[1] ?? "public"}')
		AND a.attnum > 0 AND NOT a.attisdropped
		`
				: `
	JOIN pg_catalog.pg_class c ON a.attrelid = c.oid
	JOIN pg_catalog.pg_namespace ns ON c.relnamespace = ns.oid
	WHERE c.relkind = 'r' AND a.attnum > 0 AND NOT a.attisdropped
		AND ns.nspname != 'pg_catalog' AND ns.nspname != 'information_schema'`
		}
	ORDER BY ${table ? "a.attnum" : "ns.nspname, c.relname, a.attnum"};

	`;
	} else if (input.resourceType === "ms_sql_server") {
		return `
		SELECT
    COLUMN_NAME as field,
    DATA_TYPE as DataType,
    COLUMN_DEFAULT as DefaultValue,
    CASE WHEN COLUMNPROPERTY(OBJECT_ID(TABLE_NAME), COLUMN_NAME, 'IsIdentity') = 1 THEN 'By Default' ELSE 'No' END as IsIdentity,
    CASE WHEN COLUMNPROPERTY(OBJECT_ID(TABLE_NAME), COLUMN_NAME, 'IsIdentity') = 1 THEN 1 ELSE 0 END as IsPrimaryKey, -- This line still needs correction for primary key identification
    CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,
    CASE WHEN DATA_TYPE = 'enum' THEN 1 ELSE 0 END as IsEnum${
			table ? "" : `,
		TABLE_NAME as table_name`
		}
FROM
    INFORMATION_SCHEMA.COLUMNS${
			table
				? `
WHERE
    TABLE_NAME = '${table}'`
				: ""
		}
ORDER BY
    ORDINAL_POSITION;
	`;
	} else if (
		input.resourceType === "snowflake" ||
		(input.resourceType as any) === "snowflake_oauth"
	) {
		return `
		select COLUMN_NAME as field,
		DATA_TYPE as DataType,
		COLUMN_DEFAULT as DefaultValue,
		CASE WHEN COLUMN_DEFAULT like 'AUTOINCREMENT%' THEN 'By Default' ELSE 'No' END as IsIdentity,
		CASE WHEN COLUMN_DEFAULT like 'AUTOINCREMENT%' THEN 1 ELSE 0 END as IsPrimaryKey,
		CASE WHEN IS_NULLABLE = 'YES' THEN 'YES' ELSE 'NO' END as IsNullable,
		CASE WHEN DATA_TYPE = 'enum' THEN 1 ELSE 0 END as IsEnum${
			table ? "" : `,
		table_name as table_name,
		table_schema as schema_name`
		}
	from information_schema.columns${
			table
				? `
	where table_name = '${table.split(".").reverse()[0]}' and table_schema = '${
					table.split(".").reverse()[1] ?? "PUBLIC"
				}'`
				: "\nwhere table_schema <> 'INFORMATION_SCHEMA'\n"
		}
	order by ORDINAL_POSITION;
	`;
	} else if (input.resourceType === "bigquery") {
		// TODO: find a solution for this (query uses hardcoded dataset name)
		if (!table) throw new Error("Table name is required for BigQuery");
		return `SELECT
    c.COLUMN_NAME as field,
    DATA_TYPE as DataType,
    CASE WHEN COLUMN_DEFAULT = 'NULL' THEN '' ELSE COLUMN_DEFAULT END as DefaultValue,
    CASE WHEN constraint_name is not null THEN true ELSE false END as IsPrimaryKey,
    'No' as IsIdentity,
    IS_NULLABLE as IsNullable,
    false as IsEnum
FROM
    ${table.split(".")[0]}.INFORMATION_SCHEMA.COLUMNS c
    LEFT JOIN
    ${table.split(".")[0]}.INFORMATION_SCHEMA.KEY_COLUMN_USAGE p
    on c.table_name = p.table_name AND c.column_name = p.COLUMN_NAME
WHERE
    c.TABLE_NAME = '${table.split(".")[1]}'
order by c.ORDINAL_POSITION;`;
	} else {
		throw new Error("Unsupported database type:" + input.resourceType);
	}
}

function lowercaseKeys(obj: Record<string, any>): any {
	const newObj = {};
	Object.keys(obj).forEach((key) => {
		newObj[key.toLowerCase()] = obj[key];
	});
	return newObj;
}

export async function getDbSchemas(
	resourceType: string,
	resourcePath: string,
	workspace: string | undefined,
	dbSchemas: DBSchemas,
	errorCallback: (message: string) => void,
): Promise<void> {
	if (!scripts[resourceType]) return;

	return new Promise(async (resolve, reject) => {
		if (!resourceType || !resourcePath || !workspace) {
			resolve();
			return;
		}

		const job = await JobService.runScriptPreview({
			workspace: workspace,
			requestBody: {
				language: scripts[resourceType].lang as Preview["language"],
				content: scripts[resourceType].code,
				args: {
					[scripts[resourceType].argName]:
						resourcePath.startsWith("datatable://")
							? resourcePath
							: "$res:" + resourcePath,
				},
			},
		});

		tryEvery({
			tryCode: async () => {
				if (resourcePath) {
					const testResult = await JobService.getCompletedJob({
						workspace,
						id: job,
					});
					if (!testResult.success) {
						console.error(testResult.result?.["error"]?.["message"]);
					} else {
						if (testResult.result === "WINDMILL_TOO_BIG") {
							console.info("Result is too big, fetching result separately");
							const data = await JobService.getCompletedJobResult({
								workspace,
								id: job,
							});
							testResult.result = data;
						}
						if (resourceType !== undefined) {
							if (resourceType !== "graphql") {
								const { processingFn } = scripts[resourceType];
								let schema: any;
								try {
									schema = processingFn !== undefined
										? processingFn(testResult.result)
										: testResult.result;
								} catch (e) {
									console.error(e);
									errorCallback("Error processing schema");
									resolve();
									return;
								}
								const dbSchema = {
									lang: resourceTypeToLang(resourceType) as SQLSchema["lang"],
									schema,
									publicOnly: !!schema.public || !!schema.PUBLIC ||
										!!schema.dbo,
								};
								dbSchemas[resourcePath] = {
									...dbSchema,
									stringified: stringifySchema(dbSchema),
								};
							} else {
								if (
									typeof testResult.result !== "object" ||
									!("__schema" in (testResult?.result ?? {}))
								) {
									console.error("Invalid GraphQL schema");

									errorCallback("Invalid GraphQL schema");
								} else {
									const dbSchema = {
										lang: "graphql" as GraphqlSchema["lang"],
										schema: testResult.result,
									};
									dbSchemas[resourcePath] = {
										...(dbSchema as any),
										stringified: stringifySchema(dbSchema as any),
									};
								}
							}
						}
					}
					resolve();
				}
			},
			timeoutCode: async () => {
				console.error("Could not query schema within 5s");
				errorCallback("Could not query schema within 5s");
				try {
					await JobService.cancelQueuedJob({
						workspace,
						id: job,
						requestBody: {
							reason: "Could not query schema within 5s",
						},
					});
				} catch (err) {
					console.error(err);
				}
				reject();
			},
			interval: 500,
			timeout: 5000,
		});
	});
}

export async function getTablesByResource(
	schema: Partial<Record<string, DBSchema>>,
	dbType: DbType | undefined,
	dbPath: string,
	workspace: string,
): Promise<string[]> {
	const s = Object.values(schema)?.[0];
	switch (dbType) {
		case "ms_sql_server": {
			const paths: string[] = [];
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					if (key === "dbo") {
						paths.push(`${subKey}`);
					}
				}
			}
			return paths;
		}
		case "mysql": {
			const resourceObj = (await ResourceService.getResourceValue({
				workspace,
				path: dbPath.split("$res:")[1],
			})) as any;
			const paths: string[] = [];
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					if (key === resourceObj?.database) {
						paths.push(`${subKey}`);
					} else {
						paths.push(`${key}.${subKey}`);
					}
				}
			}
			return paths;
		}
		case "snowflake": {
			const paths: string[] = [];
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					if (key === "PUBLIC") {
						paths.push(`${subKey}`);
					} else {
						paths.push(`${key}.${subKey}`);
					}
				}
			}
			return paths;
		}
		case "postgresql": {
			const paths: string[] = [];
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					if (key === "public") {
						paths.push(`${subKey}`);
					} else {
						paths.push(`${key}.${subKey}`);
					}
				}
			}
			return paths;
		}
		case "bigquery": {
			const paths: string[] = [];
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					paths.push(`${key}.${subKey}`);
				}
			}
			return paths;
		}
		case "duckdb": {
			const paths: string[] = [];
			for (const key in s?.schema) {
				for (const subKey in s.schema[key]) {
					paths.push(`${subKey}`);
				}
			}

			return paths;
		}
		default:
			return [];
	}
}

async function makeForeignKeysQuery(
	input: DbInput,
	workspace: string,
): Promise<string> {
	if (input.type === "ducklake") {
		// DuckDB/DuckLake doesn't have comprehensive foreign key support via information_schema
		return `SELECT NULL WHERE FALSE`; // Return empty result set
	} else if (input.resourceType === "postgresql") {
		return `
		SELECT
			con.conname as constraint_name,
			ns.nspname as schema_name,
			rel.relname as table_name,
			att.attname as column_name,
			ns_ref.nspname as referenced_schema_name,
			rel_ref.relname as referenced_table_name,
			att_ref.attname as referenced_column_name
		FROM pg_constraint con
		JOIN pg_class rel ON rel.oid = con.conrelid
		JOIN pg_namespace ns ON ns.oid = rel.relnamespace
		JOIN pg_class rel_ref ON rel_ref.oid = con.confrelid
		JOIN pg_namespace ns_ref ON ns_ref.oid = rel_ref.relnamespace
		JOIN unnest(con.conkey) WITH ORDINALITY AS cols(attnum, ord) ON true
		JOIN pg_attribute att ON att.attrelid = rel.oid AND att.attnum = cols.attnum
		JOIN unnest(con.confkey) WITH ORDINALITY AS refcols(attnum, ord)
			ON cols.ord = refcols.ord
		JOIN pg_attribute att_ref
			ON att_ref.attrelid = rel_ref.oid
			AND att_ref.attnum = refcols.attnum
		WHERE con.contype = 'f'
			AND ns.nspname NOT IN ('pg_catalog', 'information_schema')
		ORDER BY
			ns.nspname,
			rel.relname,
			con.conname,
			cols.ord;
		`;
	} else if (input.resourceType === "mysql") {
		const resourceObj = (await ResourceService.getResourceValue({
			workspace,
			path: input.resourcePath,
		})) as any;

		return `
		SELECT
			kcu.CONSTRAINT_NAME as constraint_name,
			kcu.TABLE_NAME as table_name,
			kcu.COLUMN_NAME as column_name,
			kcu.REFERENCED_TABLE_NAME as referenced_table_name,
			kcu.REFERENCED_COLUMN_NAME as referenced_column_name,
			kcu.TABLE_SCHEMA as schema_name,
			kcu.REFERENCED_TABLE_SCHEMA as referenced_schema_name
		FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu
		WHERE
			kcu.REFERENCED_TABLE_NAME IS NOT NULL
			AND kcu.TABLE_SCHEMA = '${resourceObj?.database ?? ""}'
		ORDER BY
			kcu.TABLE_NAME,
			kcu.CONSTRAINT_NAME,
			kcu.ORDINAL_POSITION;
		`;
	} else if (input.resourceType === "ms_sql_server") {
		return `
		SELECT
			fk.name as constraint_name,
			SCHEMA_NAME(tp.schema_id) as schema_name,
			tp.name as table_name,
			cp.name as column_name,
			SCHEMA_NAME(tr.schema_id) as referenced_schema_name,
			tr.name as referenced_table_name,
			cr.name as referenced_column_name
		FROM sys.foreign_keys fk
		JOIN sys.foreign_key_columns fkc
			ON fk.object_id = fkc.constraint_object_id
		JOIN sys.tables tp
			ON fkc.parent_object_id = tp.object_id
		JOIN sys.columns cp
			ON fkc.parent_object_id = cp.object_id
			AND fkc.parent_column_id = cp.column_id
		JOIN sys.tables tr
			ON fkc.referenced_object_id = tr.object_id
		JOIN sys.columns cr
			ON fkc.referenced_object_id = cr.object_id
			AND fkc.referenced_column_id = cr.column_id
		ORDER BY
			tp.name,
			fk.name,
			fkc.constraint_column_id;
		`;
	} else if (
		input.resourceType === "snowflake" ||
		(input.resourceType as any) === "snowflake_oauth"
	) {
		return `
		SELECT
			rc.CONSTRAINT_NAME as constraint_name,
			kcu.TABLE_SCHEMA as schema_name,
			kcu.TABLE_NAME as table_name,
			kcu.COLUMN_NAME as column_name,
			kcu2.TABLE_SCHEMA as referenced_schema_name,
			kcu2.TABLE_NAME as referenced_table_name,
			kcu2.COLUMN_NAME as referenced_column_name
		FROM INFORMATION_SCHEMA.REFERENTIAL_CONSTRAINTS rc
		JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu
			ON rc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME
		JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu2
			ON rc.UNIQUE_CONSTRAINT_NAME = kcu2.CONSTRAINT_NAME
			AND kcu.ORDINAL_POSITION = kcu2.ORDINAL_POSITION
		WHERE kcu.TABLE_SCHEMA <> 'INFORMATION_SCHEMA'
		ORDER BY
			kcu.TABLE_SCHEMA,
			kcu.TABLE_NAME,
			rc.CONSTRAINT_NAME,
			kcu.ORDINAL_POSITION;
		`;
	} else if (input.resourceType === "bigquery") {
		return `
		SELECT
			rc.CONSTRAINT_NAME as constraint_name,
			kcu.TABLE_SCHEMA as schema_name,
			kcu.TABLE_NAME as table_name,
			kcu.COLUMN_NAME as column_name,
			kcu2.TABLE_SCHEMA as referenced_schema_name,
			kcu2.TABLE_NAME as referenced_table_name,
			kcu2.COLUMN_NAME as referenced_column_name
		FROM \`region-us\`.INFORMATION_SCHEMA.REFERENTIAL_CONSTRAINTS rc
		JOIN \`region-us\`.INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu
			ON rc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME
		JOIN \`region-us\`.INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu2
			ON rc.UNIQUE_CONSTRAINT_NAME = kcu2.CONSTRAINT_NAME
			AND kcu.ORDINAL_POSITION = kcu2.ORDINAL_POSITION
		ORDER BY
			kcu.TABLE_SCHEMA,
			kcu.TABLE_NAME,
			rc.CONSTRAINT_NAME;
		`;
	} else {
		throw new Error("Unsupported database type: " + input.resourceType);
	}
}

export async function fetchAllForeignKeys(
	input: DbInput,
	workspace: string | undefined,
): Promise<ForeignKeyMetadata[] | undefined> {
	if (!input || !workspace) return undefined;

	let language = input.type == "ducklake"
		? "duckdb"
		: getLanguageByResourceType(input.resourceType);
	let content: string;

	try {
		content = await makeForeignKeysQuery(input, workspace);
	} catch (error) {
		console.error("Error creating foreign keys query:", error);
		return undefined;
	}

	const job = await JobService.runScriptPreview({
		workspace: workspace,
		requestBody: {
			language,
			content,
			args: getDatabaseArg(input),
		},
	});

	const maxRetries = 8;
	let attempts = 0;
	while (attempts < maxRetries) {
		try {
			await new Promise((resolve) =>
				setTimeout(resolve, 1000 * (attempts || 0.6))
			);

			const testResult = (await JobService.getCompletedJob({
				workspace: workspace,
				id: job,
			})) as any;

			if (testResult.success) {
				attempts = maxRetries;

				let result = testResult.result;

				// Handle MS SQL Server's nested array response
				if (
					input.type === "database" && input.resourceType === "ms_sql_server"
				) {
					result = result[0];
				}

				// Normalize keys to lowercase and return
				return result.map(lowercaseKeys) as ForeignKeyMetadata[];
			} else {
				attempts++;
			}
		} catch (error) {
			attempts++;
		}
	}

	console.error("Failed to load foreign keys after maximum retries.");
	return undefined;
}

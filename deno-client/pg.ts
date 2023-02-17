import { Client } from "https://deno.land/x/postgres@v0.17.0/mod.ts"
import { type Resource } from "./mod.ts"

/**
 * deno-postgres client API is very flexible:
 * https://deno.land/x/postgres@v0.17.0/mod.ts?s=QueryClient
 * 
 * @param db the PostgreSQL resource to generate the client for
 * 
 * @returns the client for the resource
 * 
 * @example
 * // Static query
 * ```ts
 * const { rows } = await pgClient(db).queryObject(
 *   "SELECT ID, NAME FROM CLIENTS"
 * );
 * ```
 * 
 * // Prepared Statements
 * ```ts
 * const { rows } = await pgClient(db).queryObject`SELECT ID, NAME FROM CLIENTS WHERE ID = ${id}`;
 * ```
 */
export function pgClient(
    db: Resource<"postgresql">
) {
    const databaseUrl = 'postgresql://' + db.user + ':' + db.password + '@' + db.host + ':' + db.port + '/' + db.dbname + '?sslmode=' + db.sslmode
    return new Client(databaseUrl)
}

/**
 * deno-postgres client API is very flexible:
 * https://deno.land/x/postgres@v0.17.0/mod.ts?s=QueryClient
 * 
 * @param db the PostgreSQL resource to run sql query for
 * 
 * @returns the rows corresponding to the returned objetcs
 * 
 * @example
 * // Prepared Statements
 * ```ts
 * const { rows } = await pgSql(db)`SELECT ID, NAME FROM CLIENTS WHERE ID = ${id}`;
 * ```
 */
export function pgSql(
    db: Resource<"postgresql">,
    asObjects = false
) {
    return async function queryObject(
        query: TemplateStringsArray,
        ...args: unknown[]
    ) {
        const client = pgClient(db)
        return asObjects ? await client.queryObject(query, ...args) : await client.queryArray(query, ...args)
    }
}

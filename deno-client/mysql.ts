import { createConnection } from "https://deno.land/x/mysql2@v1.0.6/mod.ts";
import { type Resource } from "./mod.ts";

/**
 * Establish MySQL connection using MySQL client for Deno:
 * https://deno.land/x/mysql2@v1.0.6/mod.ts
 *
 * IMPORTANT: make sure to close the connection with `.end()`
 *
 * @param db the MySQL resource to establish the connection for
 *
 * @returns MySQL database connection
 *
 * @example
 * ```ts
 * const conn = await mysqlClient(db);
 * await conn.execute('CREATE TABLE IF NOT EXISTS pets (name varchar(255), kind varchar(255))');
 * await conn.execute('INSERT INTO pets VALUES (?, ?)', ['behemot','cat']);
 * const [rows] = await conn.execute('SELECT * from pets');
 * conn.end();
 * console.log(rows);
 * ```
 */
export async function mysqlClient(
    db: Resource<"mysql">
) {
    const conn = await createConnection(db);
    return conn;
}

/**
 * Execute SQL query. For more info check:
 * https://deno.land/x/mysql2@v1.0.6/mod.ts
 *
 * @param db the MySQL resource to run sql query for
 *
 * @returns array with two items: rows and fields
 *
 * @example
 * ```ts
 * const kind = 'cat';
 * const { rows } = await mySql(db)`SELECT * from pets WHERE kind = ${kind}`;
 * console.log(rows);
 * ```
 */
export function mySql(
    db: Resource<"mysql">,
    asObjects = false
) {
    return async function execute(
        query: TemplateStringsArray,
        ...args: unknown[],
    ) {
        const conn = await mysqlClient(db);
        const adapter = getQueryAdapter(query, args);
        const [rows, fields] = await conn.execute(...adapter);
        conn.end();
        return { rows: asObjects ? rows : getRowsAdapter(rows), fields };
    }
}

function getQueryAdapter(template: TemplateStringsArray, args: unknown[]) {
    const text = template.reduce((curr, next) => {
        return `${curr}?${next}`;
    });
    return [text, args];
}

function getRowsAdapter(rows: object[] | object) {
    if (!Array.isArray(rows)) {
        return rows;
    }
    return rows.map((r) => Object.values(r))
}

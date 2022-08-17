import { Client } from "https://deno.land/x/postgres@v0.16.1/mod.ts"
import { type Resource, type Sql } from "./mod.ts"

export async function pgQuery(
    db: Resource<"postgresql">,
    query: Sql
) {
    db.database = db.dbname
    db.hostname = db.host

    const returned_query = await new Client(db).queryObject(query)

    return returned_query.rows
}

from typing import Any

import wmill

import psycopg2


def query(query: str, connection: str | dict[str, Any] = "g/all/postgres") -> list[tuple[Any, ...]] | None:
    """
    Query a postgres database using psycopg2 library underneath. See its documentation for more info.

    Args:
        query: The query as string, without ending ';'
        resource: The path of the resource of type 'postgres' containing the connection info.
            The default value is 'g/all/postgres'. It is by convention the default postgres
            db of any given workspace.

    Return:
        Either None if it is a non returning statement or a list of tuple for statement with return values.
    """
    if isinstance(connection, str):
        pg_con = wmill.get_resource(connection)
        if pg_con is None:
            raise Exception(f"Resource {connection} not found")
    else:
        pg_con = connection


    conn = psycopg2.connect(**pg_con)
    cur = conn.cursor()
    cur.execute(f"{query};")
    if cur.description:
        res = cur.fetchall()
    else:
        res = None
    conn.commit()
    cur.close()
    conn.close()
    return res


import os

from wmill_pg import query

def main(
        pg_con: dict, # a resource of type postgres (constrained at step 3: UI customisation)
        sql_query: str = "SELECT * from demo" 
    ):

    # query that returns rows will return them as a list
    return query(sql_query, pg_con)


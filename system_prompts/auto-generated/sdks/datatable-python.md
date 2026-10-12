## Python Datatable API (wmill)

Import: `import wmill`

# Get a DataTable client for SQL queries.
# 
# Args:
#     name: Database name (default: "main")
#     role: Connect as this data table role instead of the data table's default one.
#     as_viewer: Connect with every role of the job's viewer at once: the user of the app
#         the job runs for, or whoever ran the script. Cannot be combined with role.
# 
# Returns:
#     DataTableClient instance
def datatable(name: str = 'main', *, role: Optional[str] = None, as_viewer: bool = False) -> DataTableClient

# Client for executing SQL queries against Windmill DataTables.
class DataTableClient:
    # Initialize DataTableClient.
    # 
    # Args:
    #     client: Windmill client instance
    #     name: DataTable name
    #     role: Data table role to connect as, or None for the data table's default
    #     as_viewer: Connect with every role of the job's viewer instead of one role
    def __init__(client: Windmill, name: str, role: Optional[str] = None, as_viewer: bool = False)

    # Execute a SQL query against the DataTable.
    # 
    # Args:
    #     sql: SQL query string with $1, $2, etc. placeholders
    #     *args: Positional arguments to bind to query placeholders
    # 
    # Returns:
    #     SqlQuery instance for fetching results
    def query(sql: str, *args) -> SqlQuery


# Query result handler for DataTable and DuckLake queries.
class SqlQuery:
    # Initialize SqlQuery.
    # 
    # Args:
    #     sql: SQL query string
    #     fetch_fn: Function to execute the query
    def __init__(sql: str, fetch_fn)

    # Execute query and fetch results.
    # 
    # Args:
    #     result_collection: Optional result collection mode
    # 
    # Returns:
    #     Query results
    def fetch(result_collection: str | None = None)

    # Execute query and fetch first row of results.
    # 
    # Returns:
    #     First row of query results
    def fetch_one()

    # Execute query and fetch first row of results. Return result as a scalar value.
    # 
    # Returns:
    #     First row of query result as a scalar value
    def fetch_one_scalar()

    # Execute query and don't return any results.
    #         
    def execute()



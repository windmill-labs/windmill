# This script is used to summarize the database schema.
# You can use pg_dump to dump the schema to a file.
# pg_dump --file "schema.sql" --format=p --large-objects --schema-only --no-owner --no-privileges --no-tablespaces --no-unlogged-table-data --no-comments --no-publications --no-subscriptions --no-security-labels --no-toast-compression --no-table-access-method --verbose --schema "public" "postgresql://postgres:changeme@localhost:5432/windmill"
# Then you can run python summarize_schema.py schema.sql to get the summarized schema.

import re
import sys
from collections import defaultdict

def summarize_schema(file_path):
    """
    Parses a PostgreSQL dump file and extracts a summarized schema.
    """
    tables = defaultdict(lambda: {'columns': [], 'pks': set(), 'fks': [], 'indexes': []})
    enums = defaultdict(list)
    
    # Use state variables to parse multi-line definitions
    current_table = None
    current_enum = None
    pending_alter_table = None  # For multi-line ALTER TABLE ... ADD CONSTRAINT

    with open(file_path, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.strip()

            # --- State Resets ---
            if line.startswith(');'):
                current_table = None
                current_enum = None
                continue

            # --- Parse ENUM definitions ---
            match_enum = re.match(r"CREATE TYPE public\.(\w+) AS ENUM \($", line)
            if match_enum:
                current_enum = match_enum.group(1)
                continue
            
            if current_enum:
                # Extract enum values, which are typically like 'value',
                value = line.strip("',")
                if value and not value.startswith('--'):
                    enums[current_enum].append(value)
                continue

            # --- Parse TABLE definitions ---
            match_table = re.match(r"CREATE TABLE public\.(\w+) \($", line)
            if match_table:
                current_table = match_table.group(1)
                continue

            if current_table:
                # Parse columns within a CREATE TABLE block
                # e.g., "column_name type NOT NULL,"
                # e.g., "id bigint NOT NULL,"
                match_column = re.match(r'^"?(\w+)"?\s+([\w\d\.\[\]\(\)]+)', line)
                if match_column:
                    col_name = match_column.group(1)
                    # Skip CONSTRAINT definitions (e.g., CHECK, UNIQUE) mistakenly matched as columns
                    if col_name.upper() == 'CONSTRAINT':
                        continue
                    col_type = match_column.group(2)
                    tables[current_table]['columns'].append(f"{col_name} ({col_type})")
                
                # Parse PRIMARY KEY defined inside the table
                match_pk = re.search(r"CONSTRAINT \w+ PRIMARY KEY \((.+)\)", line)
                if match_pk:
                    # Handle multiple PK columns: "col1, col2, col3"
                    pk_cols = [p.strip().strip('"') for p in match_pk.group(1).split(',')]
                    tables[current_table]['pks'].update(pk_cols)
                continue
            
            # --- Parse Foreign Keys (defined outside CREATE TABLE, may span 2 lines) ---
            match_alter = re.match(r"ALTER TABLE ONLY public\.(\w+)$", line)
            if match_alter:
                pending_alter_table = match_alter.group(1)
                continue

            if pending_alter_table:
                match_fk = re.match(r"ADD CONSTRAINT \w+ FOREIGN KEY \(([\w,\s\"]+)\) REFERENCES public\.(\w+)\(([\w,\s\"]+)\)", line)
                if match_fk:
                    from_cols, to_table, to_cols = match_fk.groups()
                    from_cols_clean = ', '.join([c.strip().strip('"') for c in from_cols.split(',')])
                    to_cols_clean = ', '.join([c.strip().strip('"') for c in to_cols.split(',')])
                    fk_string = f"({from_cols_clean}) -> {to_table}({to_cols_clean})"
                    tables[pending_alter_table]['fks'].append(fk_string)
                pending_alter_table = None
                continue
            
            # --- Parse Index definitions ---
            match_index = re.match(r"CREATE (UNIQUE )?INDEX (\w+) ON public\.(\w+) USING (\w+) \((.+)\);", line)
            if match_index:
                is_unique = match_index.group(1) is not None
                index_name = match_index.group(2)
                table_name = match_index.group(3)
                index_type = match_index.group(4)
                columns = match_index.group(5)
                
                # Clean up column expressions
                columns_clean = columns.replace('"', '')
                
                unique_str = "UNIQUE " if is_unique else ""
                index_string = f"{unique_str}INDEX {index_name} ({index_type}) ON ({columns_clean})"
                tables[table_name]['indexes'].append(index_string)

    return enums, tables

TYPE_ABBREVIATIONS = {
    'character': 'char',
    'integer': 'int',
    'bigint': 'bigint',
    'smallint': 'smallint',
    'boolean': 'bool',
    'timestamp': 'ts',
    'bytea': 'bytes',
    'real': 'float',
    'json': 'json',
    'jsonb': 'jsonb',
    'text': 'text',
    'uuid': 'uuid',
    'bit(64)': 'bit64',
}

def shorten_type(col_str):
    """Shorten a 'name (type)' string to 'name(short_type)'."""
    match = re.match(r'^(\w+) \((.+)\)$', col_str)
    if not match:
        return col_str
    name, typ = match.group(1), match.group(2)
    # Strip public. prefix from enum types
    typ = re.sub(r'^public\.', '', typ)
    # Apply abbreviations (prefix-based to handle parametrized types like character(64) and array types like integer[])
    for prefix, abbr in TYPE_ABBREVIATIONS.items():
        if typ.startswith(prefix):
            typ = abbr + typ[len(prefix):]
            break
    return f"{name}({typ})"

def format_output(enums, tables):
    """
    Formats the parsed schema data into a compact, LLM-friendly string.
    """
    output = []

    output.append("# Database Schema")
    output.append("")
    output.append("## ENUMs")
    if not enums:
        output.append("(none)")
    else:
        for name, values in sorted(enums.items()):
            output.append(f"{name}: {', '.join(values)}")

    output.append("")
    output.append("## Tables")
    if not tables:
        output.append("(none)")
    else:
        for name, data in sorted(tables.items()):
            # Columns: inline, comma-separated, with PK marker and shortened types
            cols = []
            for col in data['columns']:
                col_short = shorten_type(col)
                col_name = col.split(' ')[0]
                if col_name in data['pks']:
                    col_short += " PK"
                cols.append(col_short)
            output.append(f"{name}: {', '.join(cols)}")

            # Foreign keys on one indented line
            if data['fks']:
                output.append(f"  FK: {' | '.join(data['fks'])}")

            # Indexes omitted for brevity — query the database for exact index info

    return "\n".join(output)

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: python {sys.argv[0]} <path_to_dump.sql>")
        sys.exit(1)
        
    input_file = sys.argv[1]
    
    try:
        enums_data, tables_data = summarize_schema(input_file)
        formatted_summary = format_output(enums_data, tables_data)
        print(formatted_summary)
    except FileNotFoundError:
        print(f"Error: The file '{input_file}' was not found.")
        sys.exit(1)
    except Exception as e:
        print(f"An unexpected error occurred: {e}")
        sys.exit(1)
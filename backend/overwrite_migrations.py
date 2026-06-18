import sys
import os
import subprocess

database_url = sys.argv[1]

print(f"database_url: {database_url}")

if database_url is None:
    print("Please provide a database url")
    sys.exit(1)

for f in os.listdir("migrations"):
    if f.endswith(".up.sql"):
        version = f.split("_")[0]
        cmd = f"cat migrations/{f} | openssl dgst -sha384 | cut -d ' ' -f 2"
        ps = subprocess.Popen(
            cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
        )
        digest = ps.communicate()[0].decode("utf-8").strip()

        cmd = f"psql '{database_url}' -c \"UPDATE _sqlx_migrations SET checksum = '\\x{digest}' WHERE version = {version};\""
        ps = subprocess.Popen(
            cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
        )
        out = ps.communicate()[0].decode("utf-8").strip()
        print(f, cmd)
        print(version, digest, out)

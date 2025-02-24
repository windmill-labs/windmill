curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker


mkdir pg_logs
chmod 777 pg_logs

sudo docker run --network=host -e POSTGRES_PASSWORD=changeme -e POSTGRES_USER=postgres -e POSTGRES_DB=windmill -e POSTGRES_INITDB_ARGS="-c log_duration=on -c log_statement=all -c log_min_duration_statement=0 -c shared_buffers=2GB -c work_mem=32MB -c effective_cache_size=4GB -c shared_preload_libraries=auto_explain -c auto_explain.log_min_duration=5 -c auto_explain.log_analyze=on -c auto_explain.log_timing=on  -c auto_explain.log_buffers=on  -c auto_explain.log_verbose=on \
                            -c log_statement=all \
                            -c log_min_duration_statement=0 \
                            -c shared_buffers=2GB \
                            -c work_mem=32MB \
                            -c effective_cache_size=4GB \
                            -c shared_preload_libraries=auto_explain \
                            -c auto_explain.log_min_duration=5 \
                            -c auto_explain.log_analyze=on \
                            -c auto_explain.log_timing=on  \
                            -c auto_explain.log_buffers=on  \
                            -c auto_explain.log_verbose=on \
                            -c auto_explain.log_nested_statements=on \
                            -c logging_collector=on \
                            -c log_directory='/var/log/postgresql' \
                            -c log_filename='postgresql.log'" \
  -v ~/pg_logs:/var/log/postgresql \
  postgres


docker run -it --network=host -e DATABASE_URL=postgres://postgres:changeme@localhost/windmill  ghcr.io/windmill-labs/windmill:main


curl -fsSL https://deno.land/install.sh | sh

cat <<EOF > suite.json
[
  {
    "kind": "noop",
    "jobs": 90000
  }
]
EOF

deno run --unstable -A -r https://raw.githubusercontent.com/windmill-labs/windmill/main/benchmarks/benchmark_suite.ts -c suite.json

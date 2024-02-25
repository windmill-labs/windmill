./substitute_ee_code.sh --dir ../windmill-ee-private
cargo sqlx prepare --workspace -- --all-targets --all-features
./substitute_ee_code.sh -r --dir ../windmill-ee-private

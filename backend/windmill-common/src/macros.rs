#[macro_export]
macro_rules! fetch_one_with_fallback {
    ($db:expr, $query_method:ident, $fetch_method:ident ,$row_type:ty, $query:literal, $table:literal || $fallback_table:literal, $( $param:expr ),* ) => {{
        let primary_query = sqlx::$query_method::<_, $row_type>(const_format::formatcp!($query, $table))
            $(.bind($param))*
            .$fetch_method($db)
            .await;

        if let Err(sqlx::Error::RowNotFound) = primary_query {
            tracing::info!("Data not found in job_params, falling back to fetching from $fallback_table");
            sqlx::$query_method::<_, $row_type>(const_format::formatcp!($query, $fallback_table))
                $(.bind($param))*
                .$fetch_method($db)
                .await
        } else {
            primary_query
        }
    }};
}

#[macro_export]
macro_rules! query_scalar_with_fallback {
    ($tx:expr, $query:literal, $fallback_query:literal,$( $param:expr ),* ) => {{
        let primary_query = sqlx::query_scalar!($query,$($param),*)
            .fetch_optional(&mut *$tx)
            .await;

        if let Err(sqlx::Error::RowNotFound) = primary_query {
            tracing::info!("Data not found in job_args, falling back to fetching from queue");
            sqlx::query_scalar!($fallback_query, $($param),*)
                .fetch_optional(&mut *$tx)
                .await
        } else {
            primary_query
        }
    }};
}

-- Create the alert_locks table
CREATE TABLE concurrency_locks (
    id VARCHAR PRIMARY KEY,
    last_locked_at TIMESTAMP NOT NULL,
    owner VARCHAR NULL
);

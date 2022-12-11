-- Add up migration script here
CREATE TABLE usage (
    id VARCHAR(50) NOT NULL,
    is_workspace BOOLEAN NOT NULL,
    month_ INTEGER NOT NULL,
    usage INTEGER NOT NULL,
    PRIMARY KEY (id, is_workspace, month_)
);
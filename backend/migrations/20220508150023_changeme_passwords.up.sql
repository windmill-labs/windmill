-- Add up migration script here
UPDATE password 
SET password_hash = '$argon2id$v=19$m=4096,t=3,p=1$oLJo/lPn/gezXCuFOEyaNw$i0T2tCkw3xUFsrBIKZwr8jVNHlIfoxQe+HfDnLtd12I'
WHERE password_hash = '$argon2id$v=19$m=4096,t=3,p=1$z0Kg3qyaS14e+YHeihkJLQ$N69flI6yQ/U98pjAHtbNxbdz2f4PrJEi9Tx1VoYk1as';

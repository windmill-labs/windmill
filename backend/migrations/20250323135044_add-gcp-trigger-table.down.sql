-- Add down migration script here
DROP TABLE gcp_trigger;
DROP TYPE DELIVERY_MODE;
DROP INDEX unique_route_path_on_push;
DROP INDEX unique_subscription_per_gcp_resource;
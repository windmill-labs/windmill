-- instance_group: add instance-level role (NULL = none, 'devops', 'superadmin')
ALTER TABLE instance_group ADD COLUMN instance_role VARCHAR(20) DEFAULT NULL;
ALTER TABLE instance_group ADD CONSTRAINT check_instance_role
  CHECK (instance_role IN ('devops', 'superadmin'));

-- password: track whether elevated role was set manually or by instance group
ALTER TABLE password ADD COLUMN role_source VARCHAR(20) NOT NULL DEFAULT 'manual';
ALTER TABLE password ADD CONSTRAINT check_role_source
  CHECK (role_source IN ('manual', 'instance_group'));

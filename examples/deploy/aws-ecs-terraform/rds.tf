resource "aws_db_subnet_group" "windmill_cluster_rds_subnets" {
  name = "windmill-cluster-rds-subnets"
  subnet_ids = [
    aws_subnet.windmill_cluster_subnet_private1.id,
    aws_subnet.windmill_cluster_subnet_private2.id,
  ]
}

resource "aws_db_instance" "windmill_cluster_rds" {
  identifier = "windmill-cluster-db"

  availability_zone     = "us-east-2a"
  instance_class        = "db.m5d.large"
  allocated_storage     = 100
  max_allocated_storage = 1000
  storage_type          = "gp3"
  # storage_throughput    = 125
  storage_encrypted = true
  # iops                  = 3000

  engine               = "postgres"
  engine_version       = "16.1"
  parameter_group_name = "default.postgres16"
  license_model        = "postgresql-license"

  db_name  = "windmill"
  username = "postgres"
  password = var.database_password

  network_type           = "IPV4"
  port                   = 5432
  db_subnet_group_name   = aws_db_subnet_group.windmill_cluster_rds_subnets.name
  vpc_security_group_ids = [aws_security_group.windmill_cluster_sg.id]
  publicly_accessible    = false

  performance_insights_enabled          = true
  performance_insights_retention_period = 7

  backup_retention_period = 7
  skip_final_snapshot     = true
  deletion_protection     = false
}
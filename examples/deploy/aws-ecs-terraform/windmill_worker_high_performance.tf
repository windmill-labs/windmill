resource "aws_launch_template" "windmill_cluster_high_performance_lt" {
  name          = "windmill-cluster-high-perf-lt"
  image_id      = "ami-09c0b8e7f21923ac0"
  instance_type = "t3.xlarge"
  # vpc_security_group_ids = [aws_security_group.windmill_cluster_sg.id]

  block_device_mappings {
    device_name = "/dev/xvda"

    ebs {
      volume_size = 100
    }
  }

  iam_instance_profile {
    name = "ecsInstanceRole"
  }

  network_interfaces {
    device_index                = 0
    delete_on_termination       = true
    associate_public_ip_address = true
    security_groups = [
      aws_security_group.windmill_cluster_sg.id
    ]
  }

  user_data = filebase64("${path.module}/lt_init_script.sh")

  tag_specifications {
    resource_type = "instance"
    tags = {
      Name = "ECS Instance - windmill-cluster"
    }
  }
}

resource "aws_autoscaling_group" "windmill_cluster_high_performance_asg" {
  name     = "windmill-cluster-high-perf-asg"
  max_size = 2
  min_size = 0

  vpc_zone_identifier = [
    aws_subnet.windmill_cluster_subnet_private1.id,
    aws_subnet.windmill_cluster_subnet_private2.id
  ]

  launch_template {
    id      = aws_launch_template.windmill_cluster_high_performance_lt.id
    version = "$Latest"
  }

  health_check_type = "EC2"

  tag {
    key                 = "AmazonECSManaged"
    value               = true
    propagate_at_launch = true
  }
}

resource "aws_ecs_capacity_provider" "windmill_cluster_high_performance_capacity_provider" {
  name = "windmill-cluster-high-perf-cp"

  auto_scaling_group_provider {
    auto_scaling_group_arn = aws_autoscaling_group.windmill_cluster_high_performance_asg.arn

    managed_scaling {
      maximum_scaling_step_size = 1
      minimum_scaling_step_size = 1
      status                    = "ENABLED"
    }
  }
}

resource "aws_cloudwatch_log_group" "windmill_cluster_windmill_high_performance_worker_log_group" {
  name = "/ecs/windmill-high-performance-worker"
}

resource "aws_ecs_task_definition" "windmill_cluster_windmill_high_performance_worker_td" {
  family             = "windmill-high-performance-worker"
  network_mode       = "awsvpc"
  execution_role_arn = data.aws_iam_role.ecs_task_execution_role.arn
  cpu                = 4096
  memory             = 15360
  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }
  requires_compatibilities = ["EC2"]

  container_definitions = jsonencode([
    {
      name      = "windmill-worker"
      image     = "ghcr.io/windmill-labs/windmill-ee:main"
      cpu       = 4096
      memory    = 15360
      essential = true
      environment = [{
        name  = "JSON_FMT"
        value = "true"
      }, {
        name  = "DATABASE_URL"
        value = "postgres://${aws_db_instance.windmill_cluster_rds.username}:${aws_db_instance.windmill_cluster_rds.password}@${aws_db_instance.windmill_cluster_rds.endpoint}/${aws_db_instance.windmill_cluster_rds.db_name}?sslmode=disable"
        }, {
        name  = "MODE"
        value = "worker"
        }, {
        name  = "WORKER_GROUP"
        value = "high_performance"
      }]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.windmill_cluster_windmill_high_performance_worker_log_group.name
          "awslogs-region"        = data.aws_region.current.name
          "awslogs-stream-prefix" = "ecs"
        }
      }
    }
  ])

  volume {
    name      = "worker_dependency_cache"
    host_path = "/tmp/windmill/cache"
  }
}

resource "aws_ecs_service" "windmill_cluster_windmill_high_performance_worker_service" {
  name            = "windmill-high-performance-worker"
  cluster         = aws_ecs_cluster.windmill_cluster.id
  task_definition = aws_ecs_task_definition.windmill_cluster_windmill_high_performance_worker_td.arn
  desired_count   = 1

  network_configuration {
    subnets = [
      aws_subnet.windmill_cluster_subnet_private1.id,
      aws_subnet.windmill_cluster_subnet_private2.id,
    ]
    security_groups = [aws_security_group.windmill_cluster_sg.id]
  }

  force_new_deployment = true
  placement_constraints {
    type = "distinctInstance"
  }

  capacity_provider_strategy {
    capacity_provider = aws_ecs_capacity_provider.windmill_cluster_high_performance_capacity_provider.name
    weight            = 100
  }

  depends_on = [aws_autoscaling_group.windmill_cluster_high_performance_asg]
}


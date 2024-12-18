resource "aws_cloudwatch_log_group" "windmill_cluster_windmill_worker_log_group" {
  name = "/ecs/windmill-worker"
}

resource "aws_ecs_task_definition" "windmill_cluster_windmill_worker_td" {
  family             = "windmill-worker"
  network_mode       = "awsvpc"
  execution_role_arn = data.aws_iam_role.ecs_task_execution_role.arn
  cpu                = 2048
  memory             = 3072
  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }
  requires_compatibilities = ["EC2"]

  container_definitions = jsonencode([
    {
      name      = "windmill-worker"
      image     = "ghcr.io/windmill-labs/windmill-ee:main"
      cpu       = 2048
      memory    = 3072
      essential = true
      environment = [{
        name  = "JSON_FMT"
        value = "true"
        }, {
        name  = "DATABASE_URL"
        value = local.db_url
        }, {
        name  = "MODE"
        value = "worker"
        }, {
        name  = "WORKER_GROUP"
        value = "default"
      }]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.windmill_cluster_windmill_worker_log_group.name
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

resource "aws_ecs_service" "windmill_cluster_windmill_worker_service" {
  name            = "windmill-worker"
  cluster         = aws_ecs_cluster.windmill_cluster.id
  task_definition = aws_ecs_task_definition.windmill_cluster_windmill_worker_td.arn
  desired_count   = 2

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
    capacity_provider = aws_ecs_capacity_provider.windmill_cluster_capacity_provider.name
    weight            = 100
  }

  depends_on = [aws_autoscaling_group.windmill_cluster_asg]
}

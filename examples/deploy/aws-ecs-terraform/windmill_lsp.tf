resource "aws_cloudwatch_log_group" "windmill_cluster_windmill_lsp_log_group" {
  name = "/ecs/windmill-lsp"
}

resource "aws_ecs_task_definition" "windmill_cluster_windmill_lsp_td" {
  family             = "windmill-lsp"
  network_mode       = "awsvpc"
  execution_role_arn = data.aws_iam_role.ecs_task_execution_role.arn
  cpu                = 1024
  memory             = 1536
  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }
  requires_compatibilities = ["EC2"]

  container_definitions = jsonencode([
    {
      name      = "windmill-lsp"
      image     = "ghcr.io/windmill-labs/windmill-lsp:latest"
      cpu       = 1024
      memory    = 1536
      essential = true
      portMappings = [
        {
          name          = "http"
          containerPort = 3001
          hostPort      = 3001
          protocol      = "tcp"
          appProtocol   = "http"
        }
      ]
      environment = [{
        name  = "JSON_FMT"
        value = "true"
      }]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.windmill_cluster_windmill_lsp_log_group.name
          "awslogs-region"        = data.aws_region.current.name
          "awslogs-stream-prefix" = "ecs"
        }
      }
    }
  ])

  volume {
    name      = "lsp_cache"
    host_path = "/root/.cache"
  }
}

resource "aws_ecs_service" "windmill_cluster_windmill_lsp_service" {
  name            = "windmill-lsp"
  cluster         = aws_ecs_cluster.windmill_cluster.id
  task_definition = aws_ecs_task_definition.windmill_cluster_windmill_lsp_td.arn
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
    capacity_provider = aws_ecs_capacity_provider.windmill_cluster_capacity_provider.name
    weight            = 100
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.windmill_cluster_windmill_lsp_tg.arn
    container_name   = "windmill-lsp"
    container_port   = 3001
  }

  depends_on = [aws_autoscaling_group.windmill_cluster_asg]
}

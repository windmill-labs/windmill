# windmill-extra: LSP, Multiplayer and Debugger in one image, behind a gateway on port 3000
# that routes /ws/* (LSP), /ws_mp/* (Multiplayer) and /ws_debug/* (Debugger).
resource "aws_cloudwatch_log_group" "windmill_cluster_windmill_extra_log_group" {
  name = "/ecs/windmill-extra"
}

resource "aws_ecs_task_definition" "windmill_cluster_windmill_extra_td" {
  family             = "windmill-extra"
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
      name      = "windmill-extra"
      image     = "ghcr.io/windmill-labs/windmill-extra:latest"
      cpu       = 1024
      memory    = 1536
      essential = true
      portMappings = [
        {
          name          = "http"
          containerPort = 3000
          hostPort      = 3000
          protocol      = "tcp"
          appProtocol   = "http"
        }
      ]
      environment = [
        {
          name  = "JSON_FMT"
          value = "true"
        },
        {
          # The target group health check (GET /) is answered by the LSP: keep it enabled, or set a
          # health_check path on windmill_cluster_windmill_extra_tg that another enabled service answers.
          name  = "ENABLE_LSP"
          value = "true"
        },
        {
          # Real-time collaboration, Enterprise Edition only.
          name  = "ENABLE_MULTIPLAYER"
          value = "true"
        },
        {
          # Keep REQUIRE_SIGNED_DEBUG_REQUESTS=true on any internet-reachable deployment.
          name  = "ENABLE_DEBUGGER"
          value = "false"
        },
        {
          name  = "REQUIRE_SIGNED_DEBUG_REQUESTS"
          value = "true"
        },
        {
          # Multiplayer and the debugger verify the tokens the Windmill server signs, with the key
          # they fetch from <WINDMILL_BASE_URL>/api/debug/jwks. Without it, every multiplayer
          # session is rejected. Use https:// if you add a TLS listener.
          name  = "WINDMILL_BASE_URL"
          value = "http://${aws_lb.windmill_cluster_alb.dns_name}"
        },
      ]
      mountPoints = [
        {
          sourceVolume  = "lsp_cache"
          containerPath = "/pyls/.cache"
        }
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.windmill_cluster_windmill_extra_log_group.name
          "awslogs-region"        = data.aws_region.current.name
          "awslogs-stream-prefix" = "ecs"
        }
      }
    }
  ])

  volume {
    name      = "lsp_cache"
    host_path = "/pyls/.cache"
  }
}

resource "aws_ecs_service" "windmill_cluster_windmill_extra_service" {
  name            = "windmill-extra"
  cluster         = aws_ecs_cluster.windmill_cluster.id
  task_definition = aws_ecs_task_definition.windmill_cluster_windmill_extra_td.arn
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
    target_group_arn = aws_lb_target_group.windmill_cluster_windmill_extra_tg.arn
    container_name   = "windmill-extra"
    container_port   = 3000
  }

  # ECS rejects a service whose target group is not yet attached to the load balancer.
  depends_on = [
    aws_autoscaling_group.windmill_cluster_asg,
    aws_lb_listener_rule.windmill_cluster_alb_extra_rule,
  ]
}

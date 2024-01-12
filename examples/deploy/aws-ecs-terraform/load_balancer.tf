resource "aws_lb_target_group" "windmill_cluster_windmill_server_tg" {
  name        = "windmill-cluster-server-tg"
  port        = 8000
  protocol    = "HTTP"
  target_type = "ip"
  vpc_id      = aws_vpc.windmill_cluster_vpc.id
}

resource "aws_lb_target_group" "windmill_cluster_windmill_lsp_tg" {
  name        = "windmill-cluster-lsp-tg"
  port        = 3001
  protocol    = "HTTP"
  target_type = "ip"
  vpc_id      = aws_vpc.windmill_cluster_vpc.id
}

resource "aws_lb_target_group" "windmill_cluster_windmill_multiplayer_tg" {
  name        = "windmill-cluster-multiplayer-tg"
  port        = 3002
  protocol    = "HTTP"
  target_type = "ip"
  vpc_id      = aws_vpc.windmill_cluster_vpc.id
}

resource "aws_lb" "windmill_cluster_alb" {
  name               = "windmill-cluster-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.windmill_cluster_sg.id]
  subnets = [
    aws_subnet.windmill_cluster_subnet_public1.id,
    aws_subnet.windmill_cluster_subnet_public2.id,
  ]

  tags = {
    Name = "windmill-cluster-alb"
  }
}

resource "aws_lb_listener" "windmill_cluster_alb_listener" {
  load_balancer_arn = aws_lb.windmill_cluster_alb.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.windmill_cluster_windmill_server_tg.arn
  }
}

resource "aws_lb_listener_rule" "windmill_cluster_alb_lsp_rule" {
  listener_arn = aws_lb_listener.windmill_cluster_alb_listener.arn
  priority     = 100

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.windmill_cluster_windmill_lsp_tg.arn
  }

  condition {
    path_pattern {
      values = ["/ws/*"]
    }
  }
}

resource "aws_lb_listener_rule" "windmill_cluster_alb_multiplayer_rule" {
  listener_arn = aws_lb_listener.windmill_cluster_alb_listener.arn
  priority     = 50

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.windmill_cluster_windmill_multiplayer_tg.arn
  }

  condition {
    path_pattern {
      values = ["/ws_mp/*"]
    }
  }
}

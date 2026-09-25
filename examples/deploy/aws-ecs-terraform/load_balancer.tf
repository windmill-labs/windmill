resource "aws_lb_target_group" "windmill_cluster_windmill_server_tg" {
  name        = "windmill-cluster-server-tg"
  port        = 8000
  protocol    = "HTTP"
  target_type = "ip"
  vpc_id      = aws_vpc.windmill_cluster_vpc.id
}

resource "aws_lb_target_group" "windmill_cluster_windmill_extra_tg" {
  # A prefix, not a fixed name: with create_before_destroy the replacement exists alongside the old
  # group for a moment, and target group names must be unique.
  name_prefix = "wmext-"
  port        = 3000
  protocol    = "HTTP"
  target_type = "ip"
  vpc_id      = aws_vpc.windmill_cluster_vpc.id

  # Replacing a target group that a listener rule uses: create the new one and repoint the rule
  # before deleting the old one, which the ALB refuses while the rule still references it.
  lifecycle {
    create_before_destroy = true
  }
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

resource "aws_lb_listener_rule" "windmill_cluster_alb_extra_rule" {
  listener_arn = aws_lb_listener.windmill_cluster_alb_listener.arn
  priority     = 100

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.windmill_cluster_windmill_extra_tg.arn
  }

  condition {
    path_pattern {
      values = ["/ws/*", "/ws_mp/*", "/ws_debug/*"]
    }
  }
}

# Upgrading a stack created before windmill-extra (standalone LSP and multiplayer services).
# The LSP rule becomes the extra rule, updated in place: it keeps priority 100, where destroying it and
# creating a new rule with the same priority can fail with PriorityInUse. The LSP target group becomes
# the extra target group, replaced before destroy (see create_before_destroy above).
moved {
  from = aws_lb_listener_rule.windmill_cluster_alb_lsp_rule
  to   = aws_lb_listener_rule.windmill_cluster_alb_extra_rule
}

moved {
  from = aws_lb_target_group.windmill_cluster_windmill_lsp_tg
  to   = aws_lb_target_group.windmill_cluster_windmill_extra_tg
}

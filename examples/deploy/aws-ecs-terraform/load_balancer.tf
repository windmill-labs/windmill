# --- Target Groups (internal HTTP — TLS terminates at ALB) ---

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

# --- ALB ---

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

# --- ACM Certificate (optional — set var.acm_certificate_arn to enable HTTPS) ---

variable "acm_certificate_arn" {
  type        = string
  default     = ""
  description = "ARN of an ACM certificate for HTTPS. If empty, ALB uses HTTP only (not recommended for production)."
}

variable "enable_https" {
  type        = bool
  default     = true
  description = "Enable HTTPS on the ALB. Requires acm_certificate_arn to be set."
}

locals {
  use_https = var.enable_https && var.acm_certificate_arn != ""
}

# --- HTTPS Listener (when certificate is provided) ---

resource "aws_lb_listener" "windmill_cluster_alb_https_listener" {
  count             = local.use_https ? 1 : 0
  load_balancer_arn = aws_lb.windmill_cluster_alb.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.acm_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.windmill_cluster_windmill_server_tg.arn
  }
}

# --- HTTP Listener (redirects to HTTPS when certificate is provided, otherwise forwards directly) ---

resource "aws_lb_listener" "windmill_cluster_alb_http_listener" {
  load_balancer_arn = aws_lb.windmill_cluster_alb.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type = local.use_https ? "redirect" : "forward"

    # Redirect to HTTPS when certificate is available
    dynamic "redirect" {
      for_each = local.use_https ? [1] : []
      content {
        port        = "443"
        protocol    = "HTTPS"
        status_code = "HTTP_301"
      }
    }

    # Forward directly when no certificate (development/testing only)
    target_group_arn = local.use_https ? null : aws_lb_target_group.windmill_cluster_windmill_server_tg.arn
  }
}

# Use the appropriate listener ARN for routing rules
locals {
  listener_arn = local.use_https ? aws_lb_listener.windmill_cluster_alb_https_listener[0].arn : aws_lb_listener.windmill_cluster_alb_http_listener.arn
}

# --- Routing Rules ---

resource "aws_lb_listener_rule" "windmill_cluster_alb_lsp_rule" {
  listener_arn = local.listener_arn
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
  listener_arn = local.listener_arn
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

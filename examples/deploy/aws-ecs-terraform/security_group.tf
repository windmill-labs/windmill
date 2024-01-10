resource "aws_security_group" "windmill_cluster_sg" {
  name        = "windmill-cluster-sg"
  description = "Windmill cluster security group"
  vpc_id      = aws_vpc.windmill_cluster_vpc.id

  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    from_port = 0
    to_port   = 0
    protocol  = "-1"
    self      = true
  }

  egress {
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  tags = {
    Name = "windmill-cluster-sg"
  }

  lifecycle {
    create_before_destroy = true
  }
}

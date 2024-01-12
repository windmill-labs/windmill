resource "aws_launch_template" "windmill_cluster_lt" {
  name          = "windmill-cluster-lt"
  image_id      = "ami-09c0b8e7f21923ac0"
  instance_type = "t3.medium"
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

resource "aws_autoscaling_group" "windmill_cluster_asg" {
  name     = "windmill-cluster-asg"
  max_size = 10
  min_size = 1

  vpc_zone_identifier = [
    aws_subnet.windmill_cluster_subnet_private1.id,
    aws_subnet.windmill_cluster_subnet_private2.id
  ]

  launch_template {
    id      = aws_launch_template.windmill_cluster_lt.id
    version = "$Latest"
  }

  health_check_type = "EC2"

  tag {
    key                 = "AmazonECSManaged"
    value               = true
    propagate_at_launch = true
  }
}

resource "aws_ecs_cluster" "windmill_cluster" {
  name = "windmill-cluster"
}

resource "aws_ecs_capacity_provider" "windmill_cluster_capacity_provider" {
  name = "windmill-cluster-capacity-provider"

  auto_scaling_group_provider {
    auto_scaling_group_arn = aws_autoscaling_group.windmill_cluster_asg.arn

    managed_scaling {
      maximum_scaling_step_size = 5
      minimum_scaling_step_size = 1
      status                    = "ENABLED"
    }
  }
}

resource "aws_ecs_cluster_capacity_providers" "windmill_cluster_cluster___capacity_provider" {
  cluster_name = aws_ecs_cluster.windmill_cluster.name

  capacity_providers = [
    aws_ecs_capacity_provider.windmill_cluster_capacity_provider.name,
    aws_ecs_capacity_provider.windmill_cluster_high_performance_capacity_provider.name # remove if not using high performance workers
  ]

  default_capacity_provider_strategy {
    base              = 1
    weight            = 100
    capacity_provider = aws_ecs_capacity_provider.windmill_cluster_capacity_provider.name
  }
}

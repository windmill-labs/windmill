resource "aws_vpc" "windmill_cluster_vpc" {
  cidr_block       = "132.0.0.0/16"
  instance_tenancy = "default"

  tags = {
    Name = "windmill-cluster-vpc"
  }
}

resource "aws_subnet" "windmill_cluster_subnet_public1" {
  vpc_id                  = aws_vpc.windmill_cluster_vpc.id
  cidr_block              = "132.0.0.0/20"
  availability_zone       = "us-east-2a"
  map_public_ip_on_launch = true

  tags = {
    Name = "windmill-cluster-subnet-public1"
  }
}

resource "aws_subnet" "windmill_cluster_subnet_public2" {
  vpc_id                  = aws_vpc.windmill_cluster_vpc.id
  cidr_block              = "132.0.16.0/20"
  availability_zone       = "us-east-2b"
  map_public_ip_on_launch = true

  tags = {
    Name = "windmill-cluster-subnet-public2"
  }
}

resource "aws_subnet" "windmill_cluster_subnet_private1" {
  vpc_id                  = aws_vpc.windmill_cluster_vpc.id
  cidr_block              = "132.0.128.0/20"
  availability_zone       = "us-east-2a"
  map_public_ip_on_launch = false

  tags = {
    Name = "windmill-cluster-subnet-private1"
  }
}

resource "aws_subnet" "windmill_cluster_subnet_private2" {
  vpc_id                  = aws_vpc.windmill_cluster_vpc.id
  cidr_block              = "132.0.144.0/20"
  availability_zone       = "us-east-2b"
  map_public_ip_on_launch = false

  tags = {
    Name = "windmill-cluster-subnet-private2"
  }
}

resource "aws_internet_gateway" "windmill_cluster_internet_gateway" {
  vpc_id = aws_vpc.windmill_cluster_vpc.id

  tags = {
    Name = "windmill-cluster-internet-gateway"
  }
}

resource "aws_eip" "windmill_cluster_nat_gateway_public1_eip" {
  vpc = true
}

resource "aws_nat_gateway" "windmill_cluster_nat_gateway_public1" {
  allocation_id = aws_eip.windmill_cluster_nat_gateway_public1_eip.id
  subnet_id     = aws_subnet.windmill_cluster_subnet_public1.id
  tags = {
    "Name" = "windmill-cluster-nat-gateway-public1"
  }
}

resource "aws_eip" "windmill_cluster_nat_gateway_public2_eip" {
  vpc = true
}

resource "aws_nat_gateway" "windmill_cluster_nat_gateway_public2" {
  allocation_id = aws_eip.windmill_cluster_nat_gateway_public2_eip.id
  subnet_id     = aws_subnet.windmill_cluster_subnet_public2.id
  tags = {
    "Name" = "windmill-cluster-nat-gateway-public2"
  }
}

resource "aws_route_table" "windmill_cluster_rtb_public" {
  vpc_id = aws_vpc.windmill_cluster_vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.windmill_cluster_internet_gateway.id
  }

  tags = {
    Name = "windmill-cluster-rtb-public"
  }
}

resource "aws_route_table_association" "windmill_cluster_subnet_public1__rtb_public" {
  subnet_id      = aws_subnet.windmill_cluster_subnet_public1.id
  route_table_id = aws_route_table.windmill_cluster_rtb_public.id
}

resource "aws_route_table_association" "windmill_cluster_subnet_public2__rtb_public" {
  subnet_id      = aws_subnet.windmill_cluster_subnet_public2.id
  route_table_id = aws_route_table.windmill_cluster_rtb_public.id
}

resource "aws_route_table" "windmill_cluster_rtb_private1" {
  vpc_id = aws_vpc.windmill_cluster_vpc.id

  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.windmill_cluster_nat_gateway_public1.id
  }

  tags = {
    Name = "windmill-cluster-rtb-private1"
  }
}

resource "aws_route_table_association" "windmill_cluster_subnet_private1__rtb_private1" {
  subnet_id      = aws_subnet.windmill_cluster_subnet_private1.id
  route_table_id = aws_route_table.windmill_cluster_rtb_private1.id
}

resource "aws_route_table" "windmill_cluster_rtb_private2" {
  vpc_id = aws_vpc.windmill_cluster_vpc.id

  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.windmill_cluster_nat_gateway_public2.id
  }

  tags = {
    Name = "windmill-cluster-rtb-private2"
  }
}

resource "aws_route_table_association" "windmill_cluster_subnet_private2__rtb_private2" {
  subnet_id      = aws_subnet.windmill_cluster_subnet_private2.id
  route_table_id = aws_route_table.windmill_cluster_rtb_private2.id
}

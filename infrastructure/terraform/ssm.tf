data "aws_ssm_parameter" "db_username" {
  name = "/ahara/db/linkdrop/username"
}

data "aws_ssm_parameter" "db_password" {
  name = "/ahara/db/linkdrop/password"
}

data "aws_ssm_parameter" "db_database" {
  name = "/ahara/db/linkdrop/database"
}

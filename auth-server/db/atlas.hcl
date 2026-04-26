env "local" {
  src = "file://schema.sql"
  url = getenv("DATABASE_URL")
  dev = "sqlite://dev?mode=memory"
}

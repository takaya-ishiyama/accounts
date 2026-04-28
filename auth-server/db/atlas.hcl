env "local" {
  src = "file://schema.sql"
  url = getenv("DATABASE_URL")
  dev = "sqlite://dev?mode=memory"
  migration {
      dir = "file://db/migrations"
  }
}

env "staging" {
  src = "file://schema.sql"
  url = getenv("DATABASE_URL")
  dev = "sqlite://dev?mode=memory"
  migration {
    dir = "file://db/migrations"
  }
}

env "production" {
  src = "file://schema.sql"
  url = getenv("DATABASE_URL")
  dev = "sqlite://dev?mode=memory"
  migration {
    dir = "file://db/migrations"
  }
}

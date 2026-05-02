use thiserror::Error;

#[derive(Debug, Clone)]
pub struct AllowedAudiences(Vec<String>);

#[derive(Debug, Error)]
pub enum AudienceError {
    #[error("allowed_audiences must not be empty")]
    Empty,
    #[error("audience '{0}' is not in the allowed list")]
    Unknown(String),
    #[error("default audience '{0}' is not in the allowed list")]
    DefaultNotInList(String),
}

pub fn parse(raw: &str) -> Result<AllowedAudiences, AudienceError> {
    let mut seen = std::collections::HashSet::new();
    let entries: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .filter(|s| seen.insert(s.clone()))
        .collect();

    if entries.is_empty() {
        return Err(AudienceError::Empty);
    }

    Ok(AllowedAudiences(entries))
}

impl AllowedAudiences {
    pub fn select<'a>(
        &'a self,
        requested: Option<&'a str>,
        default: &'a str,
    ) -> Result<&'a str, AudienceError> {
        match requested {
            Some(req) => {
                if self.0.iter().any(|a| a == req) {
                    Ok(req)
                } else {
                    Err(AudienceError::Unknown(req.to_string()))
                }
            }
            None => {
                if self.0.iter().any(|a| a == default) {
                    Ok(default)
                } else {
                    Err(AudienceError::DefaultNotInList(default.to_string()))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single() {
        let a = parse("app-api").unwrap();
        assert_eq!(a.0, vec!["app-api"]);
    }

    #[test]
    fn parse_multiple_with_whitespace() {
        let a = parse(" app-api , admin-api ").unwrap();
        assert_eq!(a.0, vec!["app-api", "admin-api"]);
    }

    #[test]
    fn parse_dedups() {
        let a = parse("a,a,b").unwrap();
        assert_eq!(a.0, vec!["a", "b"]);
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(matches!(parse(""), Err(AudienceError::Empty)));
        assert!(matches!(parse(","), Err(AudienceError::Empty)));
        assert!(matches!(parse("   "), Err(AudienceError::Empty)));
    }

    #[test]
    fn select_default_when_unspecified() {
        let a = parse("app-api").unwrap();
        assert_eq!(a.select(None, "app-api").unwrap(), "app-api");
    }

    #[test]
    fn select_known_audience() {
        let a = parse("app-api,admin-api").unwrap();
        assert_eq!(a.select(Some("admin-api"), "app-api").unwrap(), "admin-api");
    }

    #[test]
    fn select_unknown_rejected() {
        let a = parse("app-api").unwrap();
        assert!(matches!(
            a.select(Some("evil"), "app-api"),
            Err(AudienceError::Unknown(_))
        ));
    }

    #[test]
    fn select_default_not_in_list() {
        let a = parse("app-api").unwrap();
        assert!(matches!(
            a.select(None, "missing"),
            Err(AudienceError::DefaultNotInList(_))
        ));
    }
}

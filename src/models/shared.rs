use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum DocumentDirection {
    #[sqlx(rename = "outgoing")]
    Outgoing,
    #[sqlx(rename = "incoming")]
    Incoming,
}

impl DocumentDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Outgoing => "outgoing",
            Self::Incoming => "incoming",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Outgoing => "Вихідний",
            Self::Incoming => "Вхідний",
        }
    }
}

impl std::fmt::Display for DocumentDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl TryFrom<String> for DocumentDirection {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "outgoing" => Ok(Self::Outgoing),
            "incoming" => Ok(Self::Incoming),
            _ => Err(format!("Unknown direction: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_str() {
        assert_eq!(DocumentDirection::Outgoing.as_str(), "outgoing");
        assert_eq!(DocumentDirection::Incoming.as_str(), "incoming");
    }

    #[test]
    fn test_label() {
        assert_eq!(DocumentDirection::Outgoing.label(), "Вихідний");
        assert_eq!(DocumentDirection::Incoming.label(), "Вхідний");
    }

    #[test]
    fn test_display() {
        assert_eq!(DocumentDirection::Outgoing.to_string(), "Вихідний");
        assert_eq!(DocumentDirection::Incoming.to_string(), "Вхідний");
    }

    #[test]
    fn test_try_from_valid() {
        assert_eq!(
            DocumentDirection::try_from("outgoing".to_string()),
            Ok(DocumentDirection::Outgoing)
        );
        assert_eq!(
            DocumentDirection::try_from("incoming".to_string()),
            Ok(DocumentDirection::Incoming)
        );
    }

    #[test]
    fn test_try_from_invalid() {
        assert!(DocumentDirection::try_from("invalid".to_string()).is_err());
    }
}

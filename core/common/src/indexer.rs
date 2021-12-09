use serde::{de, ser};
use serde_derive::Deserialize;
use std::fmt;
use std::str::FromStr;
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IndexerName(String);

impl IndexerName {
    pub fn new(s: impl Into<String>) -> Result<Self, ()> {
        let s = s.into();

        // Note: these validation rules must be kept consistent with the validation rules
        // implemented in any other components that rely on subgraph names.

        // Enforce length limits
        if s.is_empty() || s.len() > 255 {
            return Err(());
        }

        // Check that the name contains only allowed characters.
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/')
        {
            return Err(());
        }

        // Parse into components and validate each
        for part in s.split("/") {
            // Each part must be non-empty and not too long
            if part.is_empty() || part.len() > 32 {
                return Err(());
            }

            // To keep URLs unambiguous, reserve the token "graphql"
            if part == "graphql" {
                return Err(());
            }

            // Part should not start or end with a special character.
            let first_char = part.chars().next().unwrap();
            let last_char = part.chars().last().unwrap();
            if !first_char.is_ascii_alphanumeric()
                || !last_char.is_ascii_alphanumeric()
                || !part.chars().any(|c| c.is_ascii_alphabetic())
            {
                return Err(());
            }
        }

        Ok(IndexerName(s))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for IndexerName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl ser::Serialize for IndexerName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> de::Deserialize<'de> for IndexerName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: String = de::Deserialize::deserialize(deserializer)?;
        IndexerName::new(s.clone())
            .map_err(|()| de::Error::invalid_value(de::Unexpected::Str(&s), &"valid subgraph name"))
    }
}

#[derive(Debug, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
pub enum IndexerFeature {
    nonFatalErrors,
}

impl std::fmt::Display for IndexerFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexerFeature::nonFatalErrors => write!(f, "nonFatalErrors"),
        }
    }
}

impl FromStr for IndexerFeature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "nonFatalErrors" => Ok(IndexerFeature::nonFatalErrors),
            _ => Err(anyhow::anyhow!("invalid subgraph feature {}", s)),
        }
    }
}

use serde_derive::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

#[derive(Deserialize, Serialize, Debug, Clone, Eq, Ord, PartialOrd, PartialEq)]
pub struct Url(#[serde(with = "url_serde")] pub url::Url);

impl Deref for Url {
    type Target = url::Url;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Url {
    type Err = url::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Url::parse(s)
    }
}

impl Url {
    pub fn parse(input: &str) -> Result<Url, url::ParseError> {
        let url = url::Url::parse(input)?;
        Ok(Url(url))
    }
}

use crate::error::{Error, Result};
use crate::model::*;
use crate::query::works::WorkFilter;
use crate::types::Types;
use chrono::NaiveDate;
use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;

pub mod facet;
pub mod member;
pub mod works;

pub mod filter {
    pub use super::member::MembersFilter;
    pub use super::works::WorkFilter;
}

/// filters supported for the /funders route
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FundersFilter {
    /// funders located in specified country
    Location(String),
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Open,
    Limited,
    Closed,
}

impl Visibility {
    pub fn as_str(&self) -> &str {
        match self {
            Visibility::Open => "open",
            Visibility::Limited => "limited",
            Visibility::Closed => "closed",
        }
    }
}

/// Determines how results should be sorted
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub enum Order {
    /// list results in ascending order
    Asc,
    /// list results in descending order
    Desc,
}

impl Order {
    pub fn as_str(&self) -> &str {
        match self {
            Order::Asc => "asc",
            Order::Desc => "desc",
        }
    }
}

impl CrossrefQueryParam for Order {
    fn param_key(&self) -> Cow<str> {
        Cow::Borrowed("order")
    }

    fn param_value(&self) -> Option<Cow<str>> {
        Some(Cow::Borrowed(self.as_str()))
    }
}

/// Results from a list response can be sorted by applying the sort and order parameters.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub enum Sort {
    /// Sort by relevance score
    Score,
    /// Sort by date of most recent change to metadata. Currently the same as [Deposited]
    Updated,
    /// Sort by time of most recent deposit
    Deposited,
    /// Sort by time of most recent index
    Indexed,
    /// Sort by publication date
    Published,
    /// Sort by print publication date
    PublishedPrint,
    /// Sort by online publication date
    PublishedOnline,
    /// Sort by issued date (earliest known publication date)
    Issued,
    /// Sort by number of times this DOI is referenced by other Crossref DOIs
    IsReferencedByCount,
    /// Sort by number of references included in the references section of the document identified by this DOI
    ReferenceCount,
}

impl Sort {
    pub fn as_str(&self) -> &str {
        match self {
            Sort::Score => "score",
            Sort::Updated => "updated",
            Sort::Deposited => "deposited",
            Sort::Indexed => "indexed",
            Sort::Published => "published",
            Sort::PublishedPrint => "published-print",
            Sort::PublishedOnline => "published-online",
            Sort::Issued => "issued",
            Sort::IsReferencedByCount => "is-reference-by-count",
            Sort::ReferenceCount => "reference-count",
        }
    }
}

impl CrossrefQueryParam for Sort {
    fn param_key(&self) -> Cow<str> {
        Cow::Borrowed("sort")
    }

    fn param_value(&self) -> Option<Cow<str>> {
        Some(Cow::Borrowed(self.as_str()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultControl {
    Rows(usize),
    Offset(usize),
    RowsOffset { rows: usize, offset: usize },
    Sample,
}

impl CrossrefQueryParam for ResultControl {
    fn param_key(&self) -> Cow<str> {
        match self {
            ResultControl::Rows(_) => Cow::Borrowed("rows"),
            ResultControl::Offset(_) => Cow::Borrowed("offset"),
            ResultControl::RowsOffset { rows, .. } => Cow::Owned(format!("rows={}", rows)),
            ResultControl::Sample => Cow::Borrowed("sample"),
        }
    }

    fn param_value(&self) -> Option<Cow<str>> {
        match self {
            ResultControl::Rows(r) => Some(Cow::Owned(r.to_string())),
            ResultControl::Offset(r) => Some(Cow::Owned(r.to_string())),
            ResultControl::RowsOffset { offset, .. } => {
                Some(Cow::Owned(format!("offset={}", offset)))
            }
            ResultControl::Sample => None,
        }
    }
}

/// Major resource components supported by the Crossref API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Component {
    /// returns a list of all works (journal articles, conference proceedings, books, components, etc), 20 per page
    Works,
    /// returns a list of all funders in the [Funder Registry](https://github.com/Crossref/open-funder-registry)
    Funders,
    /// returns a list of all Crossref members (mostly publishers)
    Prefixes,
    /// returns a list of valid work types
    Members,
    /// return a list of licenses applied to works in Crossref metadata
    Types,
    /// return a list of journals in the Crossref database
    Journals,
}

impl Component {
    pub fn as_str(&self) -> &str {
        match self {
            Component::Works => "works",
            Component::Funders => "funders",
            Component::Prefixes => "prefixes",
            Component::Members => "members",
            Component::Types => "types",
            Component::Journals => "journals",
        }
    }
}

impl CrossrefRoute for Component {
    fn route(&self) -> Result<String> {
        Ok(format!("/{}", self.as_str()))
    }
}

#[derive(Debug, Clone)]
pub enum ResourceComponent {
    /// a route that only addresses a single component
    Single(Component),
    /// Components can be combined with an additional `works` route
    Combined {
        primary: Component,
        identifier: String,
    },
}

impl CrossrefRoute for ResourceComponent {
    fn route(&self) -> Result<String> {
        match self {
            ResourceComponent::Single(c) => c.route(),
            ResourceComponent::Combined {
                primary,
                identifier,
            } => Ok(format!(
                "{}/{}{}",
                primary.route()?,
                identifier,
                Component::Works.route()?
            )),
        }
    }
}

pub trait Filter: ParamFragment {}

impl<T: Filter> CrossrefQueryParam for Vec<T> {
    fn param_key(&self) -> Cow<str> {
        Cow::Borrowed("filter")
    }

    fn param_value(&self) -> Option<Cow<str>> {
        Some(Cow::Owned(
            self.iter()
                .map(|x| x.fragment())
                .collect::<Vec<_>>()
                .join(","),
        ))
    }
}

pub trait ParamFragment {
    fn key(&self) -> Cow<str>;
    fn value(&self) -> Option<Cow<str>>;
    fn fragment(&self) -> Cow<str> {
        if let Some(val) = self.value() {
            Cow::Owned(format!("{}:{}", self.key(), val))
        } else {
            self.key()
        }
    }
}

pub trait CrossrefQueryParam {
    fn param_key(&self) -> Cow<str>;
    fn param_value(&self) -> Option<Cow<str>>;
    fn param(&self) -> Cow<str> {
        if let Some(val) = self.param_value() {
            Cow::Owned(format!("{}={}", self.param_key(), val))
        } else {
            self.param_key()
        }
    }
}

impl<T: AsRef<str>> CrossrefQueryParam for (T, T) {
    fn param_key(&self) -> Cow<str> {
        Cow::Borrowed(self.0.as_ref())
    }

    fn param_value(&self) -> Option<Cow<str>> {
        Some(Cow::Borrowed(self.1.as_ref()))
    }
}

pub trait CrossrefRoute {
    fn route(&self) -> Result<String>;
}

impl<T: CrossrefQueryParam> CrossrefRoute for AsRef<[T]> {
    fn route(&self) -> Result<String> {
        Ok(self
            .as_ref()
            .iter()
            .map(|x| x.param())
            .collect::<Vec<_>>()
            .join("&"))
    }
}

pub trait CrossrefQuery: CrossrefRoute {
    fn resource_component(&self) -> ResourceComponent;

    fn to_url(&self, base_path: &str) -> Result<String> {
        Ok(format!("{}{}", base_path, self.route()?))
    }

    //    fn to_json(&self) -> Result<Value> {
    //        unimplemented!()
    //    }
}

/// formats the topic for crossref by replacing all whitespaces whit `+`
pub(crate) fn format_query<T: AsRef<str>>(topic: T) -> String {
    topic
        .as_ref()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("+")
}

/// formats the individual topics of a query into the format crossref expects
/// returns a single String consisting of all words combined by '+'
pub(crate) fn format_queries<T: AsRef<str>>(topics: &[T]) -> String {
    topics
        .iter()
        .map(format_query)
        .collect::<Vec<_>>()
        .join("+")
}
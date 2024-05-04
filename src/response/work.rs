// see https://github.com/Crossref/rest-api-doc/blob/master/api_format.md

use crate::error::Result;
use crate::response::{FacetMap, QueryResponse};
use crate::{Crossref, WorkListQuery, WorksQuery};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A hashmap containing relation name, `Relation` pairs.
/// [crossref rest-api-doc](https://github.com/CrossRef/rest-api-doc/blob/master/api_format.md#relations)
/// However it seems, that the value of the relation name can also be an array.
/// Therefor the `serde_json::Value` type is used instead to prevent an invalid length error
pub type Relations = std::collections::HashMap<String, Value>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub struct WorkList {
    pub facets: FacetMap,
    /// the number of items that match the response
    pub total_results: usize,
    /// crossref responses for large number of items are divided in pages, number of elements to expect in `items`
    pub items_per_page: Option<usize>,
    /// if a query was set in the request, this will also be part in the response
    pub query: Option<QueryResponse>,
    /// all work items that are returned
    pub items: Vec<Work>,
    /// deep page through `/works` result sets
    pub next_cursor: Option<String>,
}

/// the main return type of the crossref api
/// represents a publication
/// based on the [crossref rest-api-doc](https://github.com/CrossRef/rest-api-doc/blob/master/api_format.md#work)
/// with minor adjustments
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub struct Work {
    /// Work titles, including translated titles
    pub title: Vec<String>,
    #[serde(rename = "abstract")]
    pub abstract_: Option<String>,
    /// Count of inbound references deposited with Crossref
    pub is_referenced_by_count: i32,
    /// DOI of the work
    #[serde(rename = "DOI")]
    pub doi: String,
    pub issued: PartialDate,
    pub author: Option<Vec<Contributor>>,
    pub reference: Option<Vec<Reference>>,
}

/// Helper struct to represent dates in the cross ref api as nested arrays of numbers
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct DateParts(pub Vec<Vec<Option<u32>>>);

impl DateParts {
    /// converts the nested array of numbers into the corresponding [DateField]
    /// standalone years are allowed.
    /// if an array is empty, [None] will be returned
    pub fn as_date(&self) -> Option<DateField> {
        /// converts an array of numbers into chrono [NaiveDate] if it contains at least a single value
        fn naive(v: &[Option<u32>]) -> Option<NaiveDate> {
            match v.len() {
                0 => None,
                1 => Some(NaiveDate::from_ymd(v[0]? as i32, 0, 0)),
                2 => Some(NaiveDate::from_ymd(v[0]? as i32, v[1]?, 0)),
                3 => Some(NaiveDate::from_ymd(v[0]? as i32, v[1]?, v[2]?)),
                _ => None,
            }
        }

        match self.0.len() {
            0 => None,
            1 => Some(DateField::Single(naive(&self.0[0])?)),
            2 => Some(DateField::Range {
                from: naive(&self.0[0])?,
                to: naive(&self.0[1])?,
            }),
            _ => Some(DateField::Multi(
                self.0
                    .iter()
                    .map(|x| naive(x))
                    .collect::<Option<Vec<_>>>()?,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[allow(missing_docs)]
pub struct FundingBody {
    /// Funding body primary name
    pub name: String,
    /// Optional [Open Funder Registry](http://www.crossref.org/fundingdata/registry.html) DOI uniquely identifing the funding body
    #[serde(rename = "DOI")]
    pub doi: Option<String>,
    /// Award number(s) for awards given by the funding body
    pub award: Option<Vec<String>>,
    /// Either `crossref` or `publisher`
    #[serde(rename = "doi-asserted-by")]
    pub doi_asserted_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[allow(missing_docs)]
pub struct ClinicalTrialNumber {
    /// Identifier of the clinical trial
    #[serde(rename = "clinical-trial-number")]
    pub clinical_trial_number: String,
    /// DOI of the clinical trial regsitry that assigned the trial number
    pub registry: String,
    /// One of `preResults`, `results` or `postResults`
    #[serde(rename = "type")]
    pub type_: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[allow(missing_docs)]
pub struct Contributor {
    pub family: Option<String>,
    pub given: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[allow(missing_docs)]
pub struct Affiliation {
    /// the affiliation's name
    pub name: String,
}

/// represents full date information for an item
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Date {
    /// Contains an ordered array of year, month, day of month.
    /// Only year is required. Note that the field contains a nested array,
    /// e.g. [ [ 2006, 5, 19 ] ] to conform to citeproc JSON dates
    pub date_parts: DateParts,
    /// Seconds since UNIX epoch
    pub timestamp: usize,
    /// ISO 8601 date time
    pub date_time: String,
}

impl Date {
    /// converts the nested array of numbers into the correct representation of chrono [NaiveDate]
    pub fn as_date_field(&self) -> Option<DateField> {
        self.date_parts.as_date()
    }
}

/// represents an incomplete date only consisting of year or year and month
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PartialDate {
    /// Contains an ordered array of year, month, day of month.
    /// Only year is required
    /// e.g. `[ [`2006`] ]` to conform to citeproc JSON dates
    #[serde(rename = "date-parts")]
    pub date_parts: DateParts,
}

impl PartialDate {
    /// converts the nested array of numbers into the correct representation of chrono [NaiveDate]
    pub fn as_date_field(&self) -> Option<DateField> {
        self.date_parts.as_date()
    }
}

/// Helper struct to capture all possible occurrences of dates in the crossref api, a nested Vec of numbers
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum DateField {
    /// only a single date vector
    Single(NaiveDate),
    /// two date vectors represent a range
    Range {
        /// start date of the range
        from: NaiveDate,
        /// end date of the range
        to: NaiveDate,
    },
    /// more than two date vectors are present
    Multi(Vec<NaiveDate>),
}

/// metadata about when the `Work` entry was updated
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Update {
    /// Date on which the update was published
    pub updated: PartialDate,
    /// DOI of the updated work
    #[serde(rename = "DOI")]
    pub doi: String,
    /// The type of update, for example retraction or correction
    #[serde(rename = "type")]
    pub type_: String,
    /// A display-friendly label for the update type
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[allow(missing_docs)]
pub struct Assertion {
    pub name: String,
    pub value: Option<String>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
    pub explanation: Option<String>,
    pub label: Option<String>,
    pub order: Option<i32>,
    pub group: Option<AssertionGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub struct Issue {
    /// Date on which the work was published in print
    pub published_print: Option<PartialDate>,
    /// Date on which the work was published online
    pub published_online: Option<PartialDate>,
    /// Issue number of an article's journal
    pub issue: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[allow(missing_docs)]
pub struct AssertionGroup {
    pub name: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[allow(missing_docs)]
pub struct Agency {
    pub id: String,
    pub label: Option<String>,
}

/// how the `Work` is licensed
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct License {
    /// Either `vor` (version of record,) `am` (accepted manuscript) or `unspecified`
    pub content_version: String,
    /// Number of days between the publication date of the work and the start date of this license
    pub delay_in_days: i32,
    /// Date on which this license begins to take effect
    pub start: PartialDate,
    /// Link to a web page describing this license
    #[serde(rename = "URL")]
    pub url: String,
}

/// metadata about a related resource
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceLink {
    /// Either `text-mining`, `similarity-checking` or `unspecified`
    pub intended_application: String,
    /// Either `vor` (version of record,) `am` (accepted manuscript) or `unspecified`
    pub content_version: String,
    /// Direct link to a full-text download location
    #[serde(rename = "URL")]
    pub url: String,
    /// Content type (or MIME type) of the full-text object
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub struct Reference {
    #[serde(rename = "DOI")]
    pub doi: Option<String>,
    pub year: Option<String>,
}

/// ISSN info for the `Work`
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ISSN {
    /// identifier
    pub value: String,
    /// One of `eissn`, `pissn` or `lissn`
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub struct ContentDomain {
    pub domain: Vec<String>,
    pub crossmark_restriction: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub struct Relation {
    pub id_type: Option<String>,
    pub id: Option<String>,
    pub asserted_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub struct Review {
    pub running_number: Option<String>,
    pub revision_round: Option<String>,
    /// One of `pre-publication` or `post-publication`
    pub stage: Option<String>,
    /// One of `major-revision` or `minor-revision` or `reject` or `reject-with-resubmit` or `accept`
    pub recommendation: Option<String>,
    /// One of `referee-report` or `editor-report` or `author-comment` or `community-comment` or `aggregate`
    #[serde(rename = "type")]
    pub type_: String,
    pub competing_interest_statement: Option<String>,
    pub language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::*;
    #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct Demo {
        pub date_parts: DateParts,
    }
    #[test]
    fn date_parts_serde() {
        let demo = Demo {
            date_parts: DateParts(vec![vec![Some(2017), Some(10), Some(11)]]),
        };
        let expected = r##"{"date_parts":[[2017,10,11]]}"##;
        assert_eq!(expected, &to_string(&demo).unwrap());
        assert_eq!(demo, from_str::<Demo>(expected).unwrap());
    }

    #[test]
    fn serialize_work() {
        let work_str = r##"{
    "indexed": {
      "date-parts": [
        [
          2019,
          2,
          26
        ]
      ],
      "date-time": "2019-02-26T10:43:14Z",
      "timestamp": 1551177794515
    },
    "reference-count": 105,
    "publisher": "American Psychological Association (APA)",
    "issue": "1",
    "content-domain": {
      "domain": [],
      "crossmark-restriction": false
    },
    "short-container-title": [
      "American Psychologist"
    ],
    "DOI": "10.1037/0003-066x.59.1.29",
    "type": "journal-article",
    "created": {
      "date-parts": [
        [
          2004,
          1,
          21
        ]
      ],
      "date-time": "2004-01-21T14:31:19Z",
      "timestamp": 1074695479000
    },
    "page": "29-40",
    "source": "Crossref",
    "is-referenced-by-count": 84,
    "title": [
      "How the Mind Hurts and Heals the Body."
    ],
    "prefix": "10.1037",
    "volume": "59",
    "author": [
      {
        "given": "Oakley",
        "family": "Ray",
        "sequence": "first",
        "affiliation": []
      }
    ],
    "member": "15",
    "published-online": {
      "date-parts": [
        [
          2004
        ]
      ]
    },
    "container-title": [
      "American Psychologist"
    ],
    "original-title": [],
    "language": "en",
    "link": [
      {
        "URL": "http://psycnet.apa.org/journals/amp/59/1/29.pdf",
        "content-type": "unspecified",
        "content-version": "vor",
        "intended-application": "similarity-checking"
      }
    ],
    "deposited": {
      "date-parts": [
        [
          2018,
          4,
          8
        ]
      ],
      "date-time": "2018-04-08T18:56:17Z",
      "timestamp": 1523213777000
    },
    "score": 1,
    "subtitle": [],
    "short-title": [],
    "issued": {
      "date-parts": [
        [
          null
        ]
      ]
    },
    "references-count": 105,
    "journal-issue": {
      "published-online": {
        "date-parts": [
          [
            2004
          ]
        ]
      },
      "issue": "1"
    },
    "alternative-id": [
      "2004-10043-004",
      "14736318"
    ],
    "URL": "http://dx.doi.org/10.1037/0003-066x.59.1.29",
    "relation": {},
    "ISSN": [
      "1935-990X",
      "0003-066X"
    ],
    "issn-type": [
      {
        "value": "0003-066X",
        "type": "print"
      },
      {
        "value": "1935-990X",
        "type": "electronic"
      }
    ]
  }
"##;

        let work: Work = from_str(work_str).unwrap();
    }
}

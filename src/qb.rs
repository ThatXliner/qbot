/// QBReader API client
use serde::{Deserialize, Serialize};
use url::Url;

use crate::query::ApiQuery;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Packet {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub number: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Set {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub year: u32,
    pub standard: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tossup {
    #[serde(rename = "_id")]
    pub id: String,
    pub question: String,
    pub answer: String,
    pub category: String,
    pub subcategory: String,
    pub packet: Packet,
    pub set: Set,
    #[serde(rename = "updatedAt")]
    pub updated_at: String, // Could parse to chrono::DateTime if you want
    pub difficulty: u8,
    pub number: u32,
    #[serde(rename = "answer_sanitized")]
    pub answer_sanitized: String,
    #[serde(rename = "question_sanitized")]
    pub question_sanitized: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tossups {
    pub tossups: Vec<Tossup>,
}
pub async fn random_tossup(
    reqwest: &reqwest::Client,
    api_params: &ApiQuery,
) -> Result<Tossups, reqwest::Error> {
    let mut url = Url::parse("https://www.qbreader.org/api/random-tossup").unwrap();
    for category in &api_params.categories {
        url.query_pairs_mut().append_pair("categories", category);
    }
    for subcategory in &api_params.subcategories {
        url.query_pairs_mut()
            .append_pair("subcategories", subcategory);
    }
    for alternate_subcategory in &api_params.alternate_subcategories {
        url.query_pairs_mut()
            .append_pair("alternateSubcategories", alternate_subcategory);
    }

    let response = reqwest.get(url).send().await?;
    let response = response.json::<Tossups>().await?;
    Ok(response)
}

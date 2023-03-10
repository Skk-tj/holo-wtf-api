use chrono::{DateTime, Utc};
use url::Url;
use rocket::serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, PartialEq)]
pub enum LiveFormat {
    Online,
    Irl,
    Both
}

#[derive(Debug, Serialize, PartialEq)]
pub enum Platform {
    Niconico,
    Spwn,
    Tba,
    Youtube,
    Zan,
    Zaiko,
    Other,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "tag", content = "content")]
pub enum JpyPrice {
    Tbd,
    Free,
    Fixed(i32),
    MultiTier(i32)
}

#[derive(Debug, Serialize)]
pub struct LiveConcert {
    pub id: Uuid,
    pub title: String, 
    pub format: LiveFormat,
    pub jpy_price: JpyPrice,
    pub platform: Platform,
    pub description: String,
    pub start_time: DateTime<Utc>,
    pub image_url: Option<Url>,
    pub twitter_url: Option<Url>,
    pub youtube_link: Option<Url>,
    pub ticket_link: Option<Url>,
    pub official_link: Option<Url>
}

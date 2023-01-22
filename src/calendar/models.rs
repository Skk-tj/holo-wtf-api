use chrono::{DateTime, Utc};
use url::Url;
use rocket::serde::Serialize;

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
pub enum JpyPrice {
    Tbd,
    Free,
    Fixed(i32),
    MultiTier(i32)
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Live {
    title: String,
    format: Option<LiveFormat>,
    jpy_price: JpyPrice,
    platform: Option<Platform>,
    description: String,
    image_url: Option<Url>,
    twitter_url: Option<Url>,
    ticket_url: Option<Url>,
    youtube_url: Option<Url>,
    niconico_url: Option<Url>,
    zan_url: Option<Url>,
    zaiko_url: Option<Url>
}

#[derive(Debug, Serialize)]
pub struct LiveConcert {
    pub title: String, 
    pub format: LiveFormat,
    pub jpy_price: JpyPrice,
    pub platform: Platform,
    pub description: String,
    pub start_time: DateTime<Utc>,
    pub image_url: Option<Url>,
    pub twitter_url: Option<Url>,
    pub youtube_link: Option<Url>
}
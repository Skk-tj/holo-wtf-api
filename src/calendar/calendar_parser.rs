use super::models::{LiveFormat, JpyPrice, Platform, LiveConcert};
use chrono::{DateTime, offset, NaiveTime, Utc, TimeZone};
use chrono_tz::Tz;
use regex::{Regex, RegexSet};
use icalendar::{Event, Component, DatePerhapsTime, CalendarDateTime};
use url::Url;
use log::{error, info};
use uuid::Uuid;

pub async fn get_concert_calendar_in_string() -> Result<String, reqwest::Error> {
    let res_text = reqwest::get("https://ics.teamup.com/feed/ks58vf85ajmc6pd7vu/0.ics").await?.text().await?;
    Ok(res_text)
}

pub fn get_concert_from_event(e: &Event) -> Result<LiveConcert, String> {
    let summary_str = e.get_summary()
        .ok_or("failed to get summary")
        .map_err(|e| {
            error!("{}", e);
            e.to_string()
        })?.trim();
    let category_str = e.property_value("CATEGORIES")
        .ok_or("failed to get category")
        .map_err(|e| {
            error!("{}", e);
            e.to_string()
        })?.trim();

    let (title, jpy_price, format) = get_title_price_and_platform_from_summary(summary_str)
        .map_err(|e| {
            error!("{}", e);
            e
        })?;
    let platform = get_platform_from_tag(category_str)
        .map_err(|e| {
            error!("{}", e);
            e
        })?;
    let description = e.get_description()
        .ok_or("failed to get description")
        .map_err(|e| {
            error!("{}", e);
            e.to_string()
        })?;
    let trimmed_description = remove_form_link_from_description_and_trim(String::from(description));
    let start_time = get_start_time_from_event(e)
        .map_err(|e| {
            error!("{}", e);
            e
        })?;
    let image_url: Option<Url> = get_image_url_from_event(e).map_err(|_| info!("returning null for image url")).ok();
    let twitter_url: Option<Url> = get_twitter_url_from_description(trimmed_description.as_str()).map_err(|_| info!("returning null for twitter url")).ok();
    let youtube_link: Option<Url> = get_youtube_link_from_description(trimmed_description.as_str()).map_err(|_| info!("returning null for youtube url")).ok();
    let ticket_link: Option<Url> = get_ticket_link_from_description(trimmed_description.as_str()).map_err(|_| info!("returning null for ticket url")).ok();
    let official_link: Option<Url> = get_official_link_from_description(trimmed_description.as_str()).map_err(|_| info!("returning null for official url")).ok();

    Ok(LiveConcert { id: Uuid::new_v4(), title, format, jpy_price, platform, description: trimmed_description, start_time, image_url, twitter_url, youtube_link, ticket_link, official_link })
}

pub fn get_start_time_from_event(event: &Event) -> Result<DateTime<Utc>, String> {
    let start = event.get_start();

    match start {
        Some(d) => match d {
            DatePerhapsTime::Date(naive_date) => {
                match naive_date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()).and_local_timezone(offset::Utc) {
                    offset::LocalResult::Single(t) => Ok(t),
                    _ => Err(String::from("start time unavailable"))
                }
            },
            DatePerhapsTime::DateTime(date_time) => match date_time {
                CalendarDateTime::Utc(utc) => Ok(utc),
                CalendarDateTime::Floating(naive) => {
                    match naive.and_local_timezone(offset::Utc) {
                        offset::LocalResult::Single(t) => Ok(t),
                        _ => Err(String::from("start time unavailable"))
                    }
                },
                CalendarDateTime::WithTimezone { date_time, tzid } => {
                    let tz: Tz = tzid.parse().unwrap();
                    match date_time.and_local_timezone(tz) {
                        offset::LocalResult::Single(t) => Ok(Utc.from_utc_datetime(&t.naive_utc())),
                        _ => Err(String::from("start time unavailable"))
                    }
                }
            }
        },
        _ => Err(String::from("start time unavailable"))
    }
}

pub fn get_title_price_and_platform_from_summary(summary: &str) -> Result<(String, JpyPrice, LiveFormat), String> {
    // try match "(price)(format)title" first
    let first_match = Regex::new(r"^\((.*)\)\((.*)\)(.+)$").unwrap();
    let matched = first_match.captures(summary)
        .ok_or(format!("Calendar event summary parsing failed, the text is \"{}\"", summary))?;

    let price_text = &matched[1];
    let price_parsed = get_price_from_string(price_text)?;

    let format_text = &matched[2];
    let format_parsed = get_format_from_string(format_text)?;

    let title = String::from(matched[3].trim());

    Ok((title, price_parsed, format_parsed))
}

pub fn get_price_from_string(price: &str) -> Result<JpyPrice, String> {
    if price.to_lowercase().contains("tba") || price.to_lowercase().contains("tbd")  {
        return Ok(JpyPrice::Tbd);
    }

    if price.to_lowercase().contains("free") {
        return Ok(JpyPrice::Free);
    }

    let single_tier_match = Regex::new(r"^¥(\d+)$").unwrap();

    if let Some(matched) = single_tier_match.captures(price) {
        let price_text = matched[1].to_owned();
        if let Ok(price) = price_text.parse::<i32>() {
            return Ok(JpyPrice::Fixed(price));
        } else {
            error!("Price conversion failed, the string is {}", price_text);
            return Err(String::from("Price conversion failed"));
        }
    }

    let multi_tier_match = Regex::new(r"^¥(\d+)\+$").unwrap();

    if let Some(matched) = multi_tier_match.captures(price) {
        let price_text = matched[1].to_owned();
        if let Ok(price) = price_text.parse::<i32>() {
            return Ok(JpyPrice::MultiTier(price));
        } else {
            error!("Price conversion failed, the string is {}", price_text);
            return Err(String::from("Price conversion failed"));
        }
    }

    return Err(String::from("Price conversion failed"));
}

pub fn get_format_from_string(platform: &str) -> Result<LiveFormat, String> {
    if platform.contains("🌐🪑") {
        Ok(LiveFormat::Both)
    } else if platform.contains('🌐') {
        Ok(LiveFormat::Online)
    } else if platform.contains('🪑') {
       Ok(LiveFormat::Irl)
    } else {
        error!("Live format conversion failed, the text is {}", platform);
        Err(String::from("Live format conversion failed"))
    }
}

pub fn get_platform_from_tag(tag_string: &str) -> Result<Platform, String> {
    let lowercased = tag_string.to_lowercase();

    if lowercased == "spwn" {
        Ok(Platform::Spwn)
    } else if lowercased == "youtube" {
        Ok(Platform::Youtube)
    } else if lowercased == "z-an" {
        Ok(Platform::Zan)
    } else if lowercased == "zaiko" {
        Ok(Platform::Zaiko)
    } else if lowercased == "tba" {
        Ok(Platform::Tba)
    } else if lowercased == "nico nico douga" {
        Ok(Platform::Niconico)
    } else if lowercased == "other" {
        Ok(Platform::Other)
    } else {
        error!("Calendar category parsing failed, the text is \"{}\"", tag_string);
        Err(String::from("Calendar category parsing failed"))
    }
}

pub fn get_image_url_from_description(description: &str) -> Result<Url, String> {
    let first_try_match = Regex::new(r"!Image: (https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@%_\+.~#?&//=]*))").unwrap();
    let second_try_match = Regex::new(r"!.*?: (https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@%_\+.~#?&//=]*))").unwrap();

    if let Some(matched) = first_try_match.captures(description) {
        let url = &matched[1];
        let parsed = Url::parse(url).map_err(|e| e.to_string())?;
        Ok(parsed)
    } else {
        if let Some(second_try) = second_try_match.captures(description) {
            let url = &second_try[1];
            let parsed = Url::parse(url).map_err(|e| e.to_string())?;
            return Ok(parsed);
        }

        error!("image url parse failed, the description is \"{}\"", description);
        Err(String::from("image url parse failed"))
    }
}

pub fn get_twitter_url_from_description(description: &str) -> Result<Url, String> {
    let matcher = Regex::new(r"(https?://(www\.)?twitter\.com\b([-a-zA-Z0-9()@%_\+.~#?&//=]*))").unwrap();

    if let Some(matched) = matcher.captures_iter(description).last() {
        let twitter_url = &matched[1];
        let parsed = Url::parse(twitter_url).map_err(|e| e.to_string())?;
        Ok(parsed)
    } else {
        error!("twitter url parse failed, the description is \"{}\"", description);
        Err(String::from("twitter url parse failed"))
    }
}

pub fn get_youtube_link_from_description(description: &str) -> Result<Url, String> {
    let matcher = Regex::new(r"http(?:s?)://(?:www\.)?youtu(?:be\.com/watch\?v=|\.be/)([\w\-_]*)(&(amp;)?[\w\?=]*)?").unwrap();

    if let Some(matched) = matcher.captures(description) {
        let youtube_url = &matched[0];
        let parsed = Url::parse(youtube_url).map_err(|e| e.to_string())?;
        Ok(parsed)
    } else {
        error!("youtube url parse failed, the description is \"{}\"", description);
        Err(String::from("youtube url parse failed"))
    }
}

pub fn get_ticket_link_from_description(description: &str) -> Result<Url, String> {
    let set = RegexSet::new(&[
        r"[T|t]icket (?:[L|l]ink|site):\s?(https?://(?:www\.)?[-a-zA-Z0-9@%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@%_\+.~#?&//=]*))",
        r"(https?://(www\.)?zan-live\.com\b([-a-zA-Z0-9()@%_\+.~#?&//=]*))",
        r"(https?://virtual\.spwn\.jp\b([-a-zA-Z0-9()@%_\+.~#?&//=]*))",
        r"(https?://live\.nicovideo\.jp\b([-a-zA-Z0-9()@%_\+.~#?&//=]*))"
    ]).unwrap();

    let regexes: Vec<_> = set.patterns().iter()
        .map(|pat| Regex::new(pat).unwrap())
        .collect();

    let matches: Vec<_> = set.matches(description).into_iter().collect();
    if let Some(first_idx) = matches.first() {
        if let Some(matched) = regexes[*first_idx].captures(description) {
            let ticket_url = &matched[1];
            let parsed = Url::parse(ticket_url).map_err(|e| e.to_string())?;
            Ok(parsed)
        } else {
            error!("ticket url parse failed, the description is \"{}\"", description);
            Err(String::from("ticket url parse failed"))
        }
    } else {
        error!("ticket url parse failed, the description is \"{}\"", description);
        Err(String::from("ticket url parse failed"))
    }
}

pub fn get_official_link_from_description(description: &str) -> Result<Url, String> {
    let matcher = Regex::new(r"Official site:\s?(https?://(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@%_\+.~#?&//=]*))").unwrap();

    if let Some(matched) = matcher.captures(description) {
        let official_link = &matched[1];
        let parsed = Url::parse(official_link).map_err(|e| e.to_string())?;
        Ok(parsed)
    } else {
        error!("official url parse failed, the description is \"{}\"", description);
        Err(String::from("official url parse failed"))
    }
}

pub fn get_image_url_from_event(e: &Event) -> Result<Url, String> {
    if let Some(image_url) = e.property_value("ATTACH") {
        let parsed = Url::parse(image_url).map_err(|e| e.to_string())?;
        Ok(parsed)
    } else {
        let description = e.get_description()
        .ok_or("failed to get description")
        .map_err(|e| {
            error!("{}", e);
            e.to_string()
        })?;
        let trimmed_description = remove_form_link_from_description_and_trim(String::from(description));
        get_image_url_from_description(&trimmed_description)
    }
}

pub fn remove_form_link_from_description_and_trim(description: String) -> String {
    let description_removed = description.replace(r"Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6", "");
    // let x: &[_] = &['\\', '\n'];
    let new_line_removed = description_removed.trim_end_matches("\\n");
    String::from(new_line_removed)
}

#[cfg(test)]
mod tests {
    use crate::calendar::{
        calendar_parser::{
            get_price_from_string, 
            get_format_from_string,
            get_platform_from_tag,
            get_title_price_and_platform_from_summary,
            get_image_url_from_description,
            get_twitter_url_from_description,
            get_youtube_link_from_description,
            get_ticket_link_from_description
        }, 
        models::{JpyPrice, LiveFormat, Platform},
    };
    use url::Url;

    use super::get_official_link_from_description;

    #[test]
    fn test_multi_tier_match() {
        let price_str = "¥3500+";
        assert_eq!(get_price_from_string(price_str), Ok(JpyPrice::MultiTier(3500)));
    }

    #[test]
    fn test_multi_tier_match_two() {
        let price_str = "¥5600+";
        assert_eq!(get_price_from_string(price_str), Ok(JpyPrice::MultiTier(5600)));
    }

    #[test]
    fn test_single_tier_match() {
        let price_str = "¥3500";
        assert_eq!(get_price_from_string(price_str), Ok(JpyPrice::Fixed(3500)));
    }

    #[test]
    fn test_free_tier_match() {
        let price_str = "Free";
        assert_eq!(get_price_from_string(price_str), Ok(JpyPrice::Free));
    }

    #[test]
    fn test_to_be_decided_match() {
        let price_str = "¥TBD";
        assert_eq!(get_price_from_string(price_str), Ok(JpyPrice::Tbd));
    }

    #[test]
    fn test_format_both_match() {
        let format_str = "🌐🪑";
        assert_eq!(get_format_from_string(format_str), Ok(LiveFormat::Both));
    }

    #[test]
    fn test_format_online_match() {
        let format_str = "🌐";
        assert_eq!(get_format_from_string(format_str), Ok(LiveFormat::Online));
    }

    #[test]
    fn test_format_irl_match() {
        let format_str = "🪑";
        assert_eq!(get_format_from_string(format_str), Ok(LiveFormat::Irl));
    }

    #[test]
    fn test_format_error_match() {
        let format_str = "asgaeheaf";
        assert_eq!(get_format_from_string(format_str), Err(String::from("Live format conversion failed")));
    }

    #[test]
    fn test_summary_parse() {
        let summary_str = "(¥2000+)(🌐🪑)Gaoh Omi 1st Live";
        assert_eq!(get_title_price_and_platform_from_summary(summary_str), Ok((String::from("Gaoh Omi 1st Live"), JpyPrice::MultiTier(2000), LiveFormat::Both)));
    }

    #[test]
    fn test_summary_parse_two() {
        let summary_str = "(¥5000)(🌐)Quon Tama 2nd Live";
        assert_eq!(get_title_price_and_platform_from_summary(summary_str), Ok((String::from("Quon Tama 2nd Live"), JpyPrice::Fixed(5000), LiveFormat::Online)));
    }

    #[test]
    fn test_summary_parse_three() {
        let summary_str = "(¥TBA)(🌐)LiLYPSE 4th Online Live";
        assert_eq!(get_title_price_and_platform_from_summary(summary_str), Ok((String::from("LiLYPSE 4th Online Live"), JpyPrice::Tbd, LiveFormat::Online)));
    }

    #[test]
    fn test_platform_one() {
        let platform_str = "Z-aN";
        assert_eq!(get_platform_from_tag(platform_str), Ok(Platform::Zan));
    }

    #[test]
    fn test_platform_two() {
        let platform_str = "ZAIKO";
        assert_eq!(get_platform_from_tag(platform_str), Ok(Platform::Zaiko));
    }

    #[test]
    fn test_platform_three() {
        let platform_str = "Some other";
        assert_eq!(get_platform_from_tag(platform_str), Err(String::from("Calendar category parsing failed")));
    }

    #[test]
    fn test_platform_four() {
        let platform_str = "Other";
        assert_eq!(get_platform_from_tag(platform_str), Ok(Platform::Other));
    }

    #[test]
    fn test_platform_five() {
        let platform_str = "SPWN";
        assert_eq!(get_platform_from_tag(platform_str), Ok(Platform::Spwn));
    }

    #[test]
    fn test_get_image_url_from_description_one() {
        let description = "!Image: https://pbs.twimg.com/media/FifgRAQVEAQvGVm?format=jpg&name=small\n";
        assert_eq!(get_image_url_from_description(description), Ok(Url::parse("https://pbs.twimg.com/media/FifgRAQVEAQvGVm?format=jpg&name=small").unwrap()));
    }

    #[test]
    fn test_get_image_url_from_description_two() {
        let description = "!some_images_name.jpg: https://pbs.twimg.com/media/FifgRAQVEAQvGVm?format=jpg&name=small\n";
        assert_eq!(get_image_url_from_description(description), Ok(Url::parse("https://pbs.twimg.com/media/FifgRAQVEAQvGVm?format=jpg&name=small").unwrap()));
    }

    #[test]
    fn test_get_image_url_from_description_three() {
        let description = "!some_images_name.png: https://pbs.twimg.com/media/FifgRAQVEAQvGVm?format=jpg&name=small\n";
        assert_eq!(get_image_url_from_description(description), Ok(Url::parse("https://pbs.twimg.com/media/FifgRAQVEAQvGVm?format=jpg&name=small").unwrap()));
    }

    #[test]
    fn test_get_image_url_from_description_four() {
        let description = "there is no image in the description";
        assert_eq!(get_image_url_from_description(description), Err(String::from("image url parse failed")));
    }

    #[test]
    fn test_get_image_url_from_description_five() {
        let description = r#"
        !Image: https://pbs.twimg.com/media/FnuL69caUAA-1kd?format=jpg&name=small
        
        Za-N link: https://www.zan-live.com/en/live/detail/10274[](https://www.zan-live.com/en/live/detail/10242)
        
        https://twitter.com/LiLYPSE/status/1620014088486100994
        "#;

        assert_eq!(get_image_url_from_description(description), Ok(Url::parse("https://pbs.twimg.com/media/FnuL69caUAA-1kd?format=jpg&name=small").unwrap()));
    }

    #[test]
    fn test_get_image_url_from_description_six() {
        let description = r#"
        !Fm6E28BaYAI6Ydt?format=jpg&name=small: https://pbs.twimg.com/media/Fm6E28BaYAI6Ydt?format=jpg&name=small
        "#;

        assert_eq!(get_image_url_from_description(description), Ok(Url::parse("https://pbs.twimg.com/media/Fm6E28BaYAI6Ydt?format=jpg&name=small").unwrap()));
    }

    #[test]
    fn test_get_image_url_from_description_seven() {
        let description = r#"
        !Stream Information: https://storage.zan-live.com/image/63441_ldec68lz.png 
        
        Ticket link: https://www.zan-live.com/en/live/detail/10269
        
        https://twitter.com/Yuzuha_Virtual/status/1596491181395177472
        "#;

        assert_eq!(get_image_url_from_description(description), Ok(Url::parse("https://storage.zan-live.com/image/63441_ldec68lz.png").unwrap()));
    }

    #[test]
    fn test_get_twitter_url_from_description_one() {
        let description = r#"Ticket link: https://www.zan-live.com/en/live/detail/10241

https://twitter.com/VTuberFes\\\\_jp/status/1643810761679028225: https://twitter.com/VTuberFes_jp/status/1643810761679028225

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;
        assert_eq!(get_twitter_url_from_description(description), Ok(Url::parse("https://twitter.com/VTuberFes_jp/status/1643810761679028225").unwrap()));
    }

    #[test]
    fn test_get_twitter_url_from_description_two() {
        let description = r#"SPWN link: https://virtual.spwn.jp/events/23031801-jphololive4thfes

Official site: https://hololivesuperexpo2023.hololivepro.com/fes/

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;
        assert_eq!(get_twitter_url_from_description(description), Err(String::from("twitter url parse failed")));
    }

    #[test]
    fn test_get_youtube_url_from_description_one() {
        let description = r#"YouTube link: https://www.youtube.com/watch?v=JiOw0LhFYtQ 

https://twitter.com/kaf_info/status/1616638809218895874

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;
        assert_eq!(get_youtube_link_from_description(description), Ok(Url::parse("https://www.youtube.com/watch?v=JiOw0LhFYtQ").unwrap()));
    }

    #[test]
    fn test_get_youtube_url_from_description_two() {
        let description = r#"Ticket link: https://www.zan-live.com/en/live/detail/10241

https://twitter.com/VALIS_Official/status/1588365423128420353

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;
        assert_eq!(get_youtube_link_from_description(description), Err(String::from("youtube url parse failed")));
    }

    #[test]
    fn test_get_ticket_link_from_description_one() {
        let description = r#"Ticket link: https://www.zan-live.com/en/live/detail/10241

https://twitter.com/VALIS_Official/status/1588365423128420353

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;

        assert_eq!(get_ticket_link_from_description(description), Ok(Url::parse("https://www.zan-live.com/en/live/detail/10241").unwrap()));
    }

    #[test]
    fn test_get_ticket_link_from_description_two() {
        let description = r#"SPWN link: https://virtual.spwn.jp/events/23031801-jphololive4thfes

Official site: https://hololivesuperexpo2023.hololivepro.com/fes/

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;

        assert_eq!(get_ticket_link_from_description(description), Ok(Url::parse("https://virtual.spwn.jp/events/23031801-jphololive4thfes").unwrap()));
    }

    #[test]
    fn test_get_ticket_link_from_description_three() {
        let description = r#"NND link: TBAhttps://live.nicovideo.jp/watch/lv339349141

Official site: https://enogu-official.com/230120-1/

https://twitter.com/anzu15_225/status/1616375404075548679

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;

        assert_eq!(get_ticket_link_from_description(description), Ok(Url::parse("https://live.nicovideo.jp/watch/lv339349141").unwrap()));
    }

    #[test]
    fn test_get_ticket_link_from_description_four() {
        let description = r#"(Was postponed due to technical issues)

ZaN link: https://www.zan-live.com/en/live/detail/10265

Participants: Marinasu (last appearance of Kaname Mahiro), Yumeji Nanjo, Yume Kanau, LiLYPSE, aides

https://twitter.com/MarinasuChannel/status/1596480282009686018

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;

        assert_eq!(get_ticket_link_from_description(description), Ok(Url::parse("https://www.zan-live.com/en/live/detail/10265").unwrap()));
    }

    #[test]
    fn test_get_official_link_from_description_one() {
        let description = r#"SPWN link: https://virtual.spwn.jp/events/23031801-jphololive4thfes

Official site: https://hololivesuperexpo2023.hololivepro.com/fes/: https://hololivesuperexpo2023.hololivepro.com/fes/

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;

        assert_eq!(get_official_link_from_description(description), Ok(Url::parse("https://hololivesuperexpo2023.hololivepro.com/fes/").unwrap()));
    }

    #[test]
    fn test_get_official_link_from_description_two() {
        let description = r#"Free segment: https://www.youtube.com/watch?v=8v1dGdpE484

ZAIKO link: https://palette-project.zaiko.io/item/353750

Official site: http://makeup.matereal.jp/

https://twitter.com/PaletteProject_/status/1626133117848387584

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;

        assert_eq!(get_official_link_from_description(description), Ok(Url::parse("http://makeup.matereal.jp/").unwrap()));
    }

    #[test]
    fn test_get_official_link_from_description_three() {
        let description = r#"https://twitter.com/LifeLikeaLive/status/1620013967601967107

Event Suggestion Submission form: https://forms.gle/tZwY1M19YUgUhn9i6"#;

        assert_eq!(get_official_link_from_description(description), Err(String::from("official url parse failed")));
    }
}

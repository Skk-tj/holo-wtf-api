use icalendar::{
    parser::{read_calendar, unfold},
    {Calendar, Component, Event},
    DatePerhapsTime::{Date, DateTime},
    CalendarDateTime::{Floating, Utc, WithTimezone}
};
use rocket::{
    serde::json::Json,
    response::status::NotFound
};
use chrono::offset;
use chrono_tz::{
    Tz,
    Asia::Tokyo
};
use log::warn;

mod calendar;

use crate::calendar::{
    calendar_parser::get_concert_from_event,
    calendar_parser::get_concert_calendar_in_string,
    models::LiveConcert
};

#[macro_use] extern crate rocket;

#[get("/")]
async fn index() -> Result<Json<Vec<LiveConcert>>, NotFound<String>> {
    let calendar_string = match get_concert_calendar_in_string().await {
        Ok(s) => s,
        Err(e) => return Err(NotFound(e.to_string()))
    };

    match read_calendar(unfold(calendar_string.as_str()).as_str()) {
        Ok(c) => {
            let calendar: Calendar = c.into();
            // println!("{}", calendar);
            let all_lives = calendar.components
                .iter()
                .filter_map(|c| c.as_event())
                .filter(|e| is_future_event(e))
                .filter_map(|e| get_concert_from_event(e)
                    .map_err(|err| warn!("getting concert from event failed, the error is {}, the event is {:?}", err, e))
                    .ok())
                .collect();

            Ok(Json(all_lives))
        },
        Err(e) => Err(NotFound(e))
    }
}

fn is_future_event(event: &Event) -> bool {
    let start = event.get_start();

    match start {
        Some(d) => match d {
            Date(naive_date) => naive_date > offset::Utc::now().date_naive(),
            DateTime(date_time) => match date_time {
                Utc(utc) => utc > offset::Utc::now(),
                Floating(naive) => naive.and_local_timezone(Tokyo).unwrap() > offset::Utc::now(),
                WithTimezone { date_time, tzid } => {
                    let tz: Tz = tzid.parse().unwrap();
                    match date_time.and_local_timezone(tz) {
                        offset::LocalResult::Single(t) => t > offset::Utc::now(),
                        _ => false
                    }
                }
            }
        },
        _ => false
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
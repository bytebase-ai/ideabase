use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Datelike};

// 编译时间字符串
pub fn parse_ymd(date_str: &str) -> NaiveDateTime {
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").expect("Failed to parse date");
    let time = NaiveTime::from_hms_opt(0, 0, 0);
    NaiveDateTime::new(naive_date, time.unwrap())
}
pub fn parse_ymd_hms(datetime_str: &str) -> NaiveDateTime {
    match NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        Ok(naive_datetime) => { naive_datetime },
        Err(err) => {
            log::error!("datetime.parse error {:?} {:?}", datetime_str, err);
            Local::now().naive_local()
        }
    }
}

// 获取当前日期是一年中的第几周
pub fn get_week_index() -> u32 {
    Local::now().iso_week().week()
}

// 当前时间并格式化为 "%Y-%m-%d %H:%M:%S"
pub fn get_cur_datetime() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

// 当前时间并格式化为 "%Y-%m-%d %H:%M:%S"
pub fn get_cur_date_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}
pub fn get_cur_datetime_str() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
pub fn get_cur_local_datetime() -> NaiveDateTime { Local::now().naive_local() }
pub fn format_datetime_ymd(datetime: NaiveDateTime) -> String { datetime.format("%Y-%m-%d").to_string() }
pub fn format_datetime_ymd_hms(datetime: NaiveDateTime) -> String { datetime.format("%Y-%m-%d %H:%M:%S").to_string() }

// 获取当前时间戳 - seconds
pub fn get_cur_second() -> i64 { Local::now().timestamp() }

// 获取当前时间戳 - milli
pub fn get_cur_milli() -> i64 { Local::now().timestamp_millis() }
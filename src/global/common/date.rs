use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Datelike, TimeZone};

/// 日期时间处理模块，提供各种日期时间相关的工具函数
///

// 定义常用的日期格式常量
const FORMAT_YMD: &str = "%Y-%m-%d";
const FORMAT_YMD_HMS: &str = "%Y-%m-%d %H:%M:%S";

/**
 * 获取当前本地日期时间的NaiveDateTime对象
 * 
 * @return 当前本地时间的NaiveDateTime对象
 */
#[inline]
pub fn now() -> NaiveDateTime { 
    Local::now().naive_local() 
}

/**
 * 将YYYY-MM-DD格式的日期字符串解析为NaiveDateTime对象
 * 
 * @param date_str - 格式为"YYYY-MM-DD"的日期字符串
 * @return 对应的NaiveDateTime对象，时间部分默认为00:00:00
 */
pub fn parse_date(date_str: &str) -> Result<NaiveDateTime, chrono::ParseError> {
    let naive_date = NaiveDate::parse_from_str(date_str, FORMAT_YMD)?;
    let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    Ok(NaiveDateTime::new(naive_date, time))
}

/**
 * 将YYYY-MM-DD格式的日期字符串解析为NaiveDateTime对象，解析失败时返回当前时间
 * 
 * @param date_str - 格式为"YYYY-MM-DD"的日期字符串
 * @return 对应的NaiveDateTime对象，时间部分默认为00:00:00，解析失败时返回当前时间
 */
pub fn parse_date_or_now(date_str: &str) -> NaiveDateTime {
    parse_date(date_str).unwrap_or_else(|err| {
        log::error!("date.parse error {:?} {:?}", date_str, err);
        now()
    })
}

/**
 * 将YYYY-MM-DD HH:MM:SS格式的日期时间字符串解析为NaiveDateTime对象
 * 
 * @param datetime_str - 格式为"YYYY-MM-DD HH:MM:SS"的日期时间字符串
 * @return 解析结果，成功返回NaiveDateTime对象，失败返回错误
 */
pub fn parse_datetime(datetime_str: &str) -> Result<NaiveDateTime, chrono::ParseError> {
    NaiveDateTime::parse_from_str(datetime_str, FORMAT_YMD_HMS)
}

/**
 * 将YYYY-MM-DD HH:MM:SS格式的日期时间字符串解析为NaiveDateTime对象，解析失败时返回当前时间
 * 
 * @param datetime_str - 格式为"YYYY-MM-DD HH:MM:SS"的日期时间字符串
 * @return 对应的NaiveDateTime对象，解析失败时返回当前时间
 */
pub fn parse_datetime_or_now(datetime_str: &str) -> NaiveDateTime {
    parse_datetime(datetime_str).unwrap_or_else(|err| {
        log::error!("datetime.parse error {:?} {:?}", datetime_str, err);
        now()
    })
}

/**
 * 获取当前日期是一年中的第几周
 * 
 * @return 当前日期在一年中的周数（ISO周）
 */
#[inline]
pub fn current_week() -> u32 {
    Local::now().iso_week().week()
}

/**
 * 获取当前日期并格式化为YYYY-MM-DD格式
 * 
 * @return 格式化后的当前日期字符串
 */
#[inline]
pub fn today_string() -> String {
    format_date(now())
}

/**
 * 获取当前日期时间并格式化为YYYY-MM-DD HH:MM:SS格式
 * 
 * @return 格式化后的当前日期时间字符串
 */
#[inline]
pub fn now_string() -> String {
    format_datetime(now())
}

/**
 * 将NaiveDateTime对象格式化为YYYY-MM-DD格式
 * 
 * @param datetime - 要格式化的NaiveDateTime对象
 * @return 格式化后的日期字符串
 */
#[inline]
pub fn format_date(datetime: NaiveDateTime) -> String { 
    datetime.format(FORMAT_YMD).to_string() 
}

/**
 * 将NaiveDateTime对象格式化为YYYY-MM-DD HH:MM:SS格式
 * 
 * @param datetime - 要格式化的NaiveDateTime对象
 * @return 格式化后的日期时间字符串
 */
#[inline]
pub fn format_datetime(datetime: NaiveDateTime) -> String { 
    datetime.format(FORMAT_YMD_HMS).to_string() 
}

/**
 * 获取当前时间的Unix时间戳（秒级）
 * 
 * @return 当前时间的秒级时间戳
 */
#[inline]
pub fn timestamp_seconds() -> i64 { 
    Local::now().timestamp() 
}

/**
 * 获取当前时间的Unix时间戳（毫秒级）
 * 
 * @return 当前时间的毫秒级时间戳
 */
#[inline]
pub fn timestamp_millis() -> i64 { 
    Local::now().timestamp_millis() 
}

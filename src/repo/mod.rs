use std::num::NonZero;

use time::format_description::well_known::iso8601::{Config, Iso8601, TimePrecision};

pub mod streams;

const ISO_TIME_FORMAT_CONFIG: u128 = Config::DEFAULT
    .set_time_precision(TimePrecision::Second {
        decimal_digits: NonZero::new(6),
    })
    .encode();
pub const ISO_TIME_FORMAT: Iso8601<{ ISO_TIME_FORMAT_CONFIG }> =
    Iso8601::<{ ISO_TIME_FORMAT_CONFIG }>;

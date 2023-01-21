// Taken from chrono
pub struct Parsed {
    pub year: Option<i32>,
    pub year_div_100: Option<i32>,
    pub year_mod_100: Option<i32>,
    pub isoyear: Option<i32>,
    pub isoyear_div_100: Option<i32>,
    pub isoyear_mod_100: Option<i32>,
    pub month: Option<u32>,
    pub week_from_sun: Option<u32>,
    pub week_from_mon: Option<u32>,
    pub isoweek: Option<u32>,
    // pub weekday: Option<Weekday>,
    pub ordinal: Option<u32>,
    pub day: Option<u32>,
    pub hour_div_12: Option<u32>,
    pub hour_mod_12: Option<u32>,
    pub minute: Option<u32>,
    pub second: Option<u32>,
    pub nanosecond: Option<u32>,
    pub timestamp: Option<i64>,
    pub offset: Option<i32>,
}

fn owned(x: Parsed) -> (bool, bool) {
  (true, true)
}
fn borrowed(x: &mut Parsed) -> (bool, bool) {
  (true, true)
}

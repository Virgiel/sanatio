use std::borrow::Cow;

pub use phonenumber::PhoneNumber;
pub use sanatio_derive::*;
pub use url::Url;

pub type Result<T> = std::result::Result<T, Cow<'static, str>>;

/// Trimmed non empty text of at most N bytes
pub fn max_txt<const N: usize>(str: String) -> Result<String> {
    let str = str.trim();
    if str.is_empty() {
        Err("expected a non empy string".into())
    } else if str.len() > N {
        Err(format!("expected string of a most {N}B got {}B", str.len()).into())
    } else {
        Ok(str.to_string())
    }
}

pub fn opt_max_txt<const N: usize>(str: Option<String>) -> Result<Option<String>> {
    str.map(max_txt::<N>).transpose()
}

/// Sorted list of unique number between 0 and N excluded
pub fn indexes<const N: usize>(mut v: Vec<i16>) -> Result<Vec<i16>> {
    v.sort_unstable();
    v.dedup();
    // TODO better message
    (v.len() <= N && v.iter().all(|i| *i < N as i16))
        .then(|| v)
        .ok_or("bad index".into())
}

/// Accept any input whiteout any change
pub fn pass<T>(v: T) -> Result<T> {
    Ok(v)
}

/// Email following the HTML specification
pub fn email(v: String) -> Result<String> {
    match fast_chemail::parse_email(&v) {
        Ok(_) => Ok(v),
        Err(err) => Err(err.to_string().into()),
    }
}

/// Valid phone number formatted with international convention
pub fn international_phone_number(v: String) -> Result<PhoneNumber> {
    phonenumber::parse(Some(phonenumber::country::FR), &v)
        .map_err(|err| err.to_string().into())
        .and_then(|number| {
            number
                .is_valid()
                .then(|| number)
                .ok_or("invalid phone number".into())
        })
}

/// HTTPS url
pub fn secure_url(url: url::Url) -> Result<url::Url> {
    (url.scheme() == "https")
        .then(|| url)
        .ok_or("not https".into())
}

pub fn latitude(lat: f32) -> Result<f32> {
    if (-90.0..=90.).contains(&lat) {
        Ok(lat)
    } else {
        Err(format!("Invalid latitude expected [-90,90] got {lat}").into())
    }
}

pub fn longitude(lng: f32) -> Result<f32> {
    if (-180.0..=180.).contains(&lng) {
        Ok(lng)
    } else {
        Err(format!("Invalid longitude expected [-180,180] got {lng}").into())
    }
}

pub fn opt<In, Out>(f: fn(In) -> Result<Out>) -> impl Fn(Option<In>) -> Result<Option<Out>> {
    move |v| v.map(f).transpose()
}
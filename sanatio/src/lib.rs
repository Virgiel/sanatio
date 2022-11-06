//! Generate a custom `serde` deserialization derive that validates and normalizes the data during deserialization.
//! # Examples
//! ```
//! use sanatio::{
//!     email, latitude, longitude, max_txt, pass, secure_url, opt, Url, Validate
//! };
//! use std::borrow::Cow;
//!
//! #[derive(Validate)]
//! pub struct Data {
//!    // Use a provided validation function
//!    #[validate(latitude)]
//!    pub lat: f32,
//!    // Use a dummy validation function to perform no verification
//!    #[validate(pass)]
//!    pub photo: Vec<u8>,
//!    // Use provided validation function with constant generic
//!    #[validate(max_txt::<50>)]
//!    pub name: String,
//!    // Use a custom validation function and specify a different input type than the output type
//!    #[validate(sex, String)]
//!    pub sex: i16,
//!    // Compose validation function to handle optional types
//!    #[validate(opt(secure_url))]
//!    pub link: Option<Url>,
//! }
//!
//! // Validation function errors are always strings
//! fn sex(v: String) -> Result<i16, Cow<'static, str>> {
//!    ["F", "H"].iter()
//!     .position(|s| *s == v)
//!     .ok_or_else(|| "Invalid sex index".into())
//!     .map(|pos| pos as i16)
//! }
//!
//! // You can then deserialize this type and validate its content without changing your code
//! fn usage(json: &[u8]) -> serde_json::Result<Data> {
//!     serde_json::from_slice(json)
//! }
//! ```

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

/// Sorted list of unique number between 0 and N excluded
pub fn indexes<const N: usize>(mut v: Vec<i16>) -> Result<Vec<i16>> {
    v.sort_unstable();
    v.dedup();
    // TODO better message
    (v.len() <= N && v.iter().all(|i| *i < N as i16 && *i >= 0))
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

/// Valid latitude coordinate
pub fn latitude(lat: f32) -> Result<f32> {
    if (-90.0..=90.).contains(&lat) {
        Ok(lat)
    } else {
        Err(format!("Invalid latitude expected [-90,90] got {lat}").into())
    }
}

/// Valid longitude coordinate
pub fn longitude(lng: f32) -> Result<f32> {
    if (-180.0..=180.).contains(&lng) {
        Ok(lng)
    } else {
        Err(format!("Invalid longitude expected [-180,180] got {lng}").into())
    }
}

/// Make a validated type optional
pub fn opt<In, Out>(f: fn(In) -> Result<Out>) -> impl Fn(Option<In>) -> Result<Option<Out>> {
    move |v| v.map(f).transpose()
}

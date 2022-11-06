# sanatio

Generate a custom `serde` deserialization derive that validates and normalizes the data during deserialization.
## Examples
```rust
use sanatio::{
    email, latitude, longitude, max_txt, pass, secure_url, opt, Url, Validate
};
use std::borrow::Cow;

#[derive(Validate)]
pub struct Data {
   // Use a provided validation function
   #[validate(latitude)]
   pub lat: f32,
   // Use a dummy validation function to perform no verification
   #[validate(pass)]
   pub photo: Vec<u8>,
   // Use provided validation function with constant generic
   #[validate(max_txt::<50>)]
   pub name: String,
   // Use a custom validation function and specify a different input type than the output type
   #[validate(sex, String)]
   pub sex: i16,
   // Compose validation function to handle optional types
   #[validate(opt(secure_url))]
   pub link: Option<Url>,
}

// Validation function errors are always strings
fn sex(v: String) -> Result<i16, Cow<'static, str>> {
   ["F", "H"].iter()
    .position(|s| *s == v)
    .ok_or_else(|| "Invalid sex index".into())
    .map(|pos| pos as i16)
}

// You can then deserialize this type and validate its content without changing your code
fn usage(json: &[u8]) -> serde_json::Result<Data> {
    serde_json::from_slice(json)
}
```

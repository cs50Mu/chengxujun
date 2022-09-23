mod person;

pub use person::*;

pub use self::person::person::PhoneNumber;
pub use self::person::person::PhoneType;

impl Person {
    pub fn new(name: impl Into<String>, id: i32, email: impl Into<String>, phones: impl Into<Vec<PhoneNumber>>) -> Self {
        Self {
            name: name.into(),
            id,
            email: email.into(),
            phones: phones.into(),
            ..Default::default()
        }
    }
}

impl PhoneNumber {
    pub fn new(number: impl Into<String>, phone_type: PhoneType) -> Self {
        Self {
            number: number.into(),
            phone_type: phone_type.into(),
        }
    }
}

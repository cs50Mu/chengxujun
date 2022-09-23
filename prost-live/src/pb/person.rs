/// defination for person
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Person {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    /// Unique ID number for this person.
    #[prost(int32, tag="2")]
    pub id: i32,
    #[prost(string, tag="3")]
    pub email: ::prost::alloc::string::String,
    #[prost(message, repeated, tag="4")]
    pub phones: ::prost::alloc::vec::Vec<person::PhoneNumber>,
    #[prost(bytes="vec", tag="5")]
    #[serde(skip_serializing_if="Vec::is_empty")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    #[prost(btree_map="string, int32", tag="6")]
    pub scores: ::prost::alloc::collections::BTreeMap<::prost::alloc::string::String, i32>,
}
/// Nested message and enum types in `Person`.
pub mod person {
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PhoneNumber {
        #[prost(string, tag="1")]
        pub number: ::prost::alloc::string::String,
        #[prost(enumeration="PhoneType", tag="2")]
        pub phone_type: i32,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum PhoneType {
        Mobile = 0,
        Home = 1,
        Work = 2,
    }
}
/// Our address book file is just one of these.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AddressBook {
    #[prost(message, repeated, tag="1")]
    pub people: ::prost::alloc::vec::Vec<Person>,
}

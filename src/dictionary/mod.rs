pub mod dict;
pub mod dict_resource;
pub mod loader;
pub mod system_dict;
#[cfg(test)]
pub mod system_dict_tests;
pub mod types;
pub mod user_dict;

pub use dict::{Dictionary, Matcher, RAMDictionary};
pub(crate) use dict_resource::CategoryMask;
pub use dict_resource::DictionaryResource;
pub use system_dict::SystemDictionary;
pub use types::*;
pub use user_dict::{UserDictFormat, UserDictionary};

/// Allows an enum to be serialized and deserialized into its value (stored as a single byte)
/// This is how many of the header/metadata values are stored in Nano network `Message`s
#[macro_export]
macro_rules! enum_byte {
    ($name:ident { $($variant:ident = $value:expr, )* }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[repr(u8)]
        pub enum $name {
            $($variant = $value,)*
        }

        impl $name {
            pub fn from_value(value: u8) -> Option<Self> {
                // Rust does not come with a simple way of converting a
                // number to an enum, so use a big `match`.
                match value {
                    $( $value => Some($name::$variant), )*
                    _ => None
                }
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where S: ::serde::Serializer
            {
                // Serialize the enum as a u8.
                serializer.serialize_u8(*self as u8)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
                where D: ::serde::Deserializer<'de>
            {
                struct Visitor;

                impl<'de> ::serde::de::Visitor<'de> for Visitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        formatter.write_str("byte")
                    }

                    fn visit_u8<E>(self, value: u8) -> ::std::result::Result<$name, E>
                        where E: ::serde::de::Error
                    {
                        match $name::from_value(value) {
                            Some(v) => Ok(v),
                            None => Err(E::custom(
                                format!("unknown {} value: {}",
                                stringify!($name), value)))
                        }
                    }
                }

                // Deserialize the enum from a u64.
                deserializer.deserialize_u8(Visitor)
            }
        }
    }
}
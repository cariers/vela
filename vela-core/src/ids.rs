macro_rules! def_id_serde_impls {
    ($struct_name:ident) => {
        impl serde::Serialize for $struct_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                self.as_str().serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                s.parse::<Self>().map_err(::serde::de::Error::custom)
            }
        }
    };
    ($struct_name:ident, _) => {};
}

macro_rules! def_id {
    ($struct_name:ident: String) => {
        #[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
        pub struct $struct_name(smol_str::SmolStr);
        impl $struct_name {
            /// Extracts a string slice containing the entire id.
            #[inline(always)]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl PartialEq<str> for $struct_name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<&str> for $struct_name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<String> for $struct_name {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other
            }
        }

        impl PartialOrd for $struct_name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $struct_name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.as_str().cmp(other.as_str())
            }
        }

        impl AsRef<str> for $struct_name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::ops::Deref for $struct_name {
            type Target = str;

            fn deref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $struct_name {
            type Err = ParseIdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok($struct_name(s.into()))
            }
        }

        impl serde::Serialize for $struct_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: serde::ser::Serializer
            {
                self.as_str().serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: serde::de::Deserializer<'de>
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                s.parse::<Self>().map_err(::serde::de::Error::custom)
            }
        }
    };
    ($struct_name:ident, $prefix:literal $(| $alt_prefix:literal)* $(, { $generate_hint:tt })?) => {
        #[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
        pub struct $struct_name(smol_str::SmolStr);

        impl $struct_name {

            #[inline(always)]
            pub fn prefixes() -> &'static [&'static str] {
                &[$prefix$(, $alt_prefix)*]
            }

            #[inline(always)]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn is_valid_prefix(prefix: &str) -> bool {
                prefix == $prefix $( || prefix == $alt_prefix )*
            }
        }

        impl PartialEq<str> for $struct_name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<&str> for $struct_name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<String> for $struct_name {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other
            }
        }

        impl PartialOrd for $struct_name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $struct_name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.as_str().cmp(other.as_str())
            }
        }

        impl AsRef<str> for $struct_name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::ops::Deref for $struct_name {
            type Target = str;

            fn deref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $struct_name {
            type Err = ParseIdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if !s.starts_with($prefix) $(
                    && !s.starts_with($alt_prefix)
                )* {
                    Err(ParseIdError {
                        typename: stringify!($struct_name),
                        expected: stringify!(id to start with $prefix $(or $alt_prefix)*),
                    })
                } else {
                    Ok($struct_name(s.into()))
                }
            }
        }

        def_id_serde_impls!($struct_name $(, $generate_hint )*);
    };

    (#[optional] enum $enum_name:ident { $( $variant_name:ident($($variant_type:tt)*) ),* $(,)* }) => {
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub enum $enum_name {
            None,
            $( $variant_name($($variant_type)*), )*
        }

        impl $enum_name {
            pub fn as_str(&self) -> &str {
                match *self {
                    $enum_name::None => "",
                    $( $enum_name::$variant_name(ref id) => id.as_str(), )*
                }
            }
        }

        impl PartialEq<str> for $enum_name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<&str> for $enum_name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<String> for $enum_name {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other
            }
        }

        impl AsRef<str> for $enum_name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::ops::Deref for $enum_name {
            type Target = str;

            fn deref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match *self {
                    $enum_name::None => Ok(()),
                    $( $enum_name::$variant_name(ref id) => id.fmt(f), )*
                }
            }
        }

        impl std::default::Default for $enum_name {
            fn default() -> Self {
                $enum_name::None
            }
        }

        impl std::str::FromStr for $enum_name {
            type Err = ParseIdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let prefix = s.find('_')
                    .map(|i| &s[0..=i])
                    .ok_or_else(|| ParseIdError {
                        typename: stringify!($enum_name),
                        expected: "id to start with a prefix (as in 'prefix_')"
                    })?;

                match prefix {
                    $(_ if $($variant_type)*::is_valid_prefix(prefix) => {
                        Ok($enum_name::$variant_name(s.parse()?))
                    })*
                    _ => {
                        Err(ParseIdError {
                            typename: stringify!($enum_name),
                            expected: "unknown id prefix",
                        })
                    }
                }
            }
        }

        impl serde::Serialize for $enum_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: serde::ser::Serializer
            {
                self.as_str().serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $enum_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: serde::de::Deserializer<'de>
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                s.parse::<Self>().map_err(::serde::de::Error::custom)
            }
        }

        $(
            impl From<$($variant_type)*> for $enum_name {
                fn from(id: $($variant_type)*) -> Self {
                    $enum_name::$variant_name(id)
                }
            }
        )*
    };
    (enum $enum_name:ident { $( $(#[$test:meta])? $variant_name:ident($($variant_type:tt)*) ),+ $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        #[derive(SmartDefault)]
        pub enum $enum_name {
            $( $(#[$test])* $variant_name($($variant_type)*), )*
        }

        impl $enum_name {
            pub fn as_str(&self) -> &str {
                match *self {
                    $( $enum_name::$variant_name(ref id) => id.as_str(), )*
                }
            }
        }

        impl PartialEq<str> for $enum_name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<&str> for $enum_name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<String> for $enum_name {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other
            }
        }

        impl AsRef<str> for $enum_name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }


        impl std::ops::Deref for $enum_name {
            type Target = str;

            fn deref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match *self {
                    $( $enum_name::$variant_name(ref id) => id.fmt(f), )*
                }
            }
        }

        impl std::str::FromStr for $enum_name {
            type Err = ParseIdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let prefix = s.find('_')
                    .map(|i| &s[0..=i])
                    .ok_or_else(|| ParseIdError {
                        typename: stringify!($enum_name),
                        expected: "id to start with a prefix (as in 'prefix_')"
                    })?;

                match prefix {
                    $(_ if $($variant_type)*::is_valid_prefix(prefix) => {
                        Ok($enum_name::$variant_name(s.parse()?))
                    })*
                    _ => {
                        Err(ParseIdError {
                            typename: stringify!($enum_name),
                            expected: "unknown id prefix",
                        })
                    }
                }
            }
        }

        impl serde::Serialize for $enum_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: serde::ser::Serializer
            {
                self.as_str().serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $enum_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: serde::de::Deserializer<'de>
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                s.parse::<Self>().map_err(::serde::de::Error::custom)
            }
        }

        $(
            impl From<$($variant_type)*> for $enum_name {
                fn from(id: $($variant_type)*) -> Self {
                    $enum_name::$variant_name(id)
                }
            }
        )*
    };

}

#[derive(Clone, Debug)]
pub struct ParseIdError {
    typename: &'static str,
    expected: &'static str,
}

impl std::fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid `{}`, expected {}", self.typename, self.expected)
    }
}

impl std::error::Error for ParseIdError {
    fn description(&self) -> &str {
        "error parsing an id"
    }
}

def_id!(SessionId, "ss_");
def_id!(PlayerId, "py_");

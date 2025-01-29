use serde::{Deserialize, Serialize};
use sqlx::postgres::PgTypeInfo;
use sqlx::Postgres;

#[derive(Copy, Clone, Eq)]
pub struct Varchar<const N: usize> {
    characters: [u8; N],
    len: usize,
}

impl<const N: usize> Varchar<N> {
    pub fn from_str(s: &str) -> Option<Self> {
        let len = s.as_bytes().len();
        if len > N {
            return None;
        }
        let mut characters = [0; N];
        (&mut characters[..len]).copy_from_slice(&s.as_bytes()[..len]);
        Some(Self { characters, len })
    }

    pub fn as_str(&self) -> &str {
        match std::str::from_utf8(&self.characters[..self.len]) {
            Ok(s) => s,
            Err(_) => unreachable!(),
        }
    }
}

impl<const N: usize> Default for Varchar<N> {
    fn default() -> Self {
        Self { characters: [0; N], len: 0 }
    }
}

impl<const N: usize> From<&str> for Varchar<N> {
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap()
    }
}

impl<const N: usize> From<String> for Varchar<N> {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap()
    }
}

impl<const N: usize> Into<String> for Varchar<N> {
    fn into(self) -> String {
        self.as_str().to_string()
    }
}

impl<const N: usize> AsRef<str> for Varchar<N> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const N: usize> std::ops::Deref for Varchar<N> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<const N: usize> std::fmt::Display for Varchar<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}

impl<const N: usize> std::fmt::Debug for Varchar<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<const N: usize> PartialEq<str> for Varchar<N> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<const N: usize, T: AsRef<str>> PartialEq<T> for Varchar<N> {
    fn eq(&self, other: &T) -> bool {
        self.as_str() == other.as_ref()
    }
}

impl<const N: usize> Serialize for Varchar<N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_ref().serialize(serializer)
    }
}

impl<'de, const N: usize> Deserialize<'de> for Varchar<N> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <&str>::deserialize(deserializer)?;
        Self::from_str(s)
            .ok_or_else(|| <D::Error as serde::de::Error>::invalid_length(s.len(), &"varchar"))
    }
}

impl<const N: usize> sqlx::Type<Postgres> for Varchar<N> {
    fn type_info() -> PgTypeInfo {
        <&str as sqlx::Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <&str as sqlx::Type<Postgres>>::compatible(ty)
    }
}

impl<'r, const N: usize> sqlx::Decode<'r, Postgres> for Varchar<N> {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as sqlx::Decode<Postgres>>::decode(value)?;
        Ok(Self::from_str(s).ok_or_else(|| "varchar: decoded string too long")?)
    }
}

impl<'q, const N: usize> sqlx::Encode<'q, Postgres> for Varchar<N> {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <&str as sqlx::Encode<Postgres>>::encode_by_ref(&self.as_ref(), buf)
    }
}

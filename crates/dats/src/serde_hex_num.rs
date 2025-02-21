use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};

pub trait SerdeBytesFunctions<const N: usize> {
    fn serde_to_le_bytes(&self) -> [u8; N];
    fn serde_from_le_bytes(arr: [u8; N]) -> Self;
}

impl SerdeBytesFunctions<1> for u8 {
    fn serde_to_le_bytes(&self) -> [u8; 1] {
        self.to_le_bytes()
    }

    fn serde_from_le_bytes(arr: [u8; 1]) -> Self {
        Self::from_le_bytes(arr)
    }
}

impl SerdeBytesFunctions<2> for u16 {
    fn serde_to_le_bytes(&self) -> [u8; 2] {
        self.to_le_bytes()
    }

    fn serde_from_le_bytes(arr: [u8; 2]) -> Self {
        Self::from_le_bytes(arr)
    }
}

impl SerdeBytesFunctions<4> for u32 {
    fn serde_to_le_bytes(&self) -> [u8; 4] {
        self.to_le_bytes()
    }

    fn serde_from_le_bytes(arr: [u8; 4]) -> Self {
        Self::from_le_bytes(arr)
    }
}

pub fn serialize<T: SerdeBytesFunctions<N>, const N: usize, S: Serializer>(
    v: &T,
    s: S,
) -> Result<S::Ok, S::Error> {
    let hex = format!(
        "0x{}",
        v.serde_to_le_bytes()
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<String>()
    );
    String::serialize(&hex, s)
}

pub fn deserialize<'de, T: SerdeBytesFunctions<N>, const N: usize, D: Deserializer<'de>>(
    d: D,
) -> Result<T, D::Error> {
    let hex = String::deserialize(d)?;
    Ok(T::serde_from_le_bytes(
        (2..hex.len())
            .step_by(2)
            .map(|idx| u8::from_str_radix(&hex[idx..idx + 2], 16).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    ))
}

use crate::apu::ApuState;
use crate::memory::AddressSpace;
use crate::startup::EmulationState;
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::{fs, io};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SaveStateError {
    #[error("error serializing/deserializing state: {source}")]
    Serialization {
        #[from]
        source: bincode::Error,
    },
    #[error("error reading/writing state: {source}")]
    FileSystem {
        #[from]
        source: io::Error,
    },
    #[error("error determining save state path: {msg}")]
    FilePath { msg: String },
}

pub fn serialize_array<S, T, const N: usize>(
    array: &[T; N],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let mut state = serializer.serialize_tuple(N)?;
    for value in array {
        state.serialize_element(value)?;
    }
    state.end()
}

struct DeserializeArrayVisitor<T, const N: usize> {
    marker: PhantomData<T>,
}

impl<T, const N: usize> DeserializeArrayVisitor<T, N> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de, T, const N: usize> Visitor<'de> for DeserializeArrayVisitor<T, N>
where
    T: Deserialize<'de> + Default + Copy,
{
    type Value = [T; N];

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "an array of size {}", N)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut array = [T::default(); N];

        for (i, value) in array.iter_mut().enumerate() {
            let Some(elem) = seq.next_element()?
            else {
                return Err(de::Error::custom(format!(
                    "expected array to have {} elements, only got {}",
                    N, i
                )));
            };

            *value = elem;
        }

        if seq.next_element::<T>()?.is_some() {
            return Err(de::Error::custom(format!(
                "array has more than {} elements",
                N
            )));
        }

        Ok(array)
    }
}

pub fn deserialize_array<'de, D, T, const N: usize>(deserializer: D) -> Result<[T; N], D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default + Copy,
{
    deserializer.deserialize_tuple(N, DeserializeArrayVisitor::new())
}

pub fn serialize_2d_array<S, T, const N: usize, const M: usize>(
    value: &[[T; M]; N],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let mut state = serializer.serialize_tuple(M * N)?;
    for row in value {
        for value in row {
            state.serialize_element(value)?;
        }
    }
    state.end()
}

struct Deserialize2dArrayVisitor<T, const N: usize, const M: usize> {
    marker: PhantomData<T>,
}

impl<T, const N: usize, const M: usize> Deserialize2dArrayVisitor<T, N, M> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<'de, T, const N: usize, const M: usize> Visitor<'de> for Deserialize2dArrayVisitor<T, N, M>
where
    T: Deserialize<'de> + Default + Copy,
{
    type Value = [[T; M]; N];

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a 2D array with {} rows and {} cols", N, M)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut array = [[T::default(); M]; N];

        for row in array.iter_mut() {
            for value in row.iter_mut() {
                let Some(elem) = seq.next_element()?
                else {
                    return Err(de::Error::custom(format!("array has fewer than {}*{} elements", M, N)));
                };
                *value = elem;
            }
        }

        if seq.next_element::<T>()?.is_some() {
            return Err(de::Error::custom(format!(
                "array has more than {}*{} elements",
                M, N
            )));
        }

        Ok(array)
    }
}

pub fn deserialize_2d_array<'de, D, T, const N: usize, const M: usize>(
    deserializer: D,
) -> Result<[[T; M]; N], D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default + Copy,
{
    deserializer.deserialize_tuple(M * N, Deserialize2dArrayVisitor::new())
}

pub fn determine_save_state_path(gb_file_path: &str) -> Result<PathBuf, SaveStateError> {
    let Some(parent) = Path::new(gb_file_path).parent()
    else {
        return Err(SaveStateError::FilePath { msg: format!("unable to determine parent dir of {gb_file_path}") });
    };

    let Some(file_name_ss_ext) = Path::new(gb_file_path).with_extension("ss0").file_name().and_then(|file_name| file_name.to_str().map(String::from))
    else {
        return Err(SaveStateError::FilePath { msg: format!("unable to determine file name of {gb_file_path}") });
    };

    Ok(parent.join(Path::new(&file_name_ss_ext)))
}

pub fn save_state<P>(state: &EmulationState, path: P) -> Result<(), SaveStateError>
where
    P: AsRef<Path>,
{
    let serialized_state = bincode::serialize(state)?;
    fs::write(path.as_ref(), serialized_state)?;

    log::info!(
        "Successfully wrote save state to '{}'",
        path.as_ref().display()
    );

    Ok(())
}

pub fn load_state<P>(
    path: P,
    existing_apu_state: ApuState,
    existing_address_space: &AddressSpace,
) -> Result<EmulationState, (SaveStateError, Box<ApuState>)>
where
    P: AsRef<Path>,
{
    let serialized_state = match fs::read(path.as_ref()) {
        Ok(serialized_state) => serialized_state,
        Err(err) => {
            return Err((err.into(), Box::new(existing_apu_state)));
        }
    };
    let mut state: EmulationState = match bincode::deserialize(&serialized_state) {
        Ok(state) => state,
        Err(err) => {
            return Err((err.into(), Box::new(existing_apu_state)));
        }
    };

    state
        .address_space
        .copy_cartridge_rom_from(existing_address_space);
    state
        .apu_state
        .move_unserializable_fields_from(existing_apu_state);

    log::info!(
        "Successfully loaded save state from '{}'",
        path.as_ref().display()
    );

    Ok(state)
}
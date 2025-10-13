use std::marker::PhantomData;

use matrix_sdk::bytes::Buf;
use serde::{ de::DeserializeOwned, Serialize };

/// Typed & simplified wrapper around [`sled::Tree`]
#[derive(Clone, Debug)]
pub struct State<V: Serialize + DeserializeOwned>(sled::Tree, PhantomData<V>);

impl<V: Serialize + DeserializeOwned> State<V> {
    pub(crate) fn new(tree: sled::Tree) -> Self {
        Self(tree, PhantomData)
    }

    /// Inserts a record into State
    pub fn insert(&self, key: impl AsRef<str>, value: impl Into<V>) -> crate::Result<Option<V>> {
        let key = key.as_ref().as_bytes();
        let mut serialized: Vec<u8> = vec![];
        ciborium::into_writer(&value.into(), &mut serialized)?;
        let previous = self.0.insert(key, serialized)?;
        if let Some(prev) = previous {
            Ok(Some(ciborium::from_reader::<V, _>(prev.reader())?))
        } else {
            Ok(None)
        }
    }

    /// Tries to get a record by key
    pub fn get(&self, key: impl AsRef<str>) -> crate::Result<Option<V>> {
        let key = key.as_ref().as_bytes();
        if let Some(record) = self.0.get(key)? {
            Ok(Some(ciborium::from_reader::<V, _>(record.reader())?))
        } else {
            Ok(None)
        }
    }

    /// Gets the name of this State
    pub fn name(&self) -> String {
        String::from_utf8(self.0.name().to_vec()).unwrap()
    }

    /// Deletes a key from this State
    pub fn remove(&self, key: impl AsRef<str>) -> crate::Result<Option<V>> {
        let key = key.as_ref().as_bytes();
        if let Some(record) = self.0.remove(key)? {
            Ok(Some(ciborium::from_reader::<V, _>(record.reader())?))
        } else {
            Ok(None)
        }
    }

    /// Returns an iterator over all keys in this State
    pub fn keys(&self) -> impl Iterator<Item = String> {
        self.0
            .iter()
            .keys()
            .filter_map(|k| {
                if let Ok(key) = k { Some(String::from_utf8(key.to_vec()).unwrap()) } else { None }
            })
    }

    /// Flushes this State
    pub fn flush(&self) -> crate::Result<usize> {
        Ok(self.0.flush()?)
    }
}

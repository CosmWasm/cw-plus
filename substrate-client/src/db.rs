use std::io;

use kvdb::{DBOp, DBTransaction, DBValue, KeyValueDB};
use parity_scale_codec::alloc::collections::{BTreeMap, HashMap};
use parity_scale_codec::{Decode, Encode, Error, Input, Output};
use parity_util_mem::MallocSizeOf;
use parking_lot::RwLock;

use crate::genesis::GenesisData;
use parity_scale_codec::alloc::sync::Arc;

#[derive(Encode, Decode, Clone)]
pub struct Data {
    pub db: DB,
    pub genesis_data: GenesisData,
}

#[derive(Default, MallocSizeOf)]
pub struct DB {
    columns: Arc<RwLock<HashMap<u32, BTreeMap<Vec<u8>, DBValue>>>>,
}

pub fn create(num_cols: u32) -> DB {
    let mut cols = HashMap::new();

    for idx in 0..num_cols {
        cols.insert(idx, BTreeMap::new());
    }

    DB {
        columns: Arc::new(RwLock::new(cols)),
    }
}

impl Clone for DB {
    fn clone(&self) -> Self {
        Self {
            columns: self.columns.clone(),
        }
    }
}

impl KeyValueDB for DB {
    fn get(&self, col: u32, key: &[u8]) -> io::Result<Option<DBValue>> {
        let columns = self.columns.read();
        match columns.get(&col) {
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("No such column family: {:?}", col),
            )),
            Some(map) => Ok(map.get(key).cloned()),
        }
    }

    fn get_by_prefix(&self, col: u32, prefix: &[u8]) -> Option<Box<[u8]>> {
        let columns = self.columns.read();
        match columns.get(&col) {
            None => None,
            Some(map) => map
                .iter()
                .find(|&(ref k, _)| k.starts_with(prefix))
                .map(|(_, v)| v.to_vec().into_boxed_slice()),
        }
    }

    fn write_buffered(&self, transaction: DBTransaction) {
        let mut columns = self.columns.write();
        let ops = transaction.ops;
        for op in ops {
            match op {
                DBOp::Insert { col, key, value } => {
                    if let Some(col) = columns.get_mut(&col) {
                        col.insert(key.into_vec(), value);
                    }
                }
                DBOp::Delete { col, key } => {
                    if let Some(col) = columns.get_mut(&col) {
                        col.remove(&*key);
                    }
                }
            }
        }
    }

    fn flush(&self) -> io::Result<()> {
        Ok(())
    }

    fn iter<'a>(&'a self, col: u32) -> Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a> {
        match self.columns.read().get(&col) {
            Some(map) => Box::new(
                // TODO: Maybe need to optimize
                map.clone()
                    .into_iter()
                    .map(|(k, v)| (k.into_boxed_slice(), v.into_boxed_slice())),
            ),
            None => Box::new(None.into_iter()),
        }
    }

    fn iter_from_prefix<'a>(
        &'a self,
        col: u32,
        prefix: &'a [u8],
    ) -> Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a> {
        match self.columns.read().get(&col) {
            Some(map) => Box::new(
                map.clone()
                    .into_iter()
                    .filter(move |&(ref k, _)| k.starts_with(prefix))
                    .map(|(k, v)| (k.into_boxed_slice(), v.into_boxed_slice())),
            ),
            None => Box::new(None.into_iter()),
        }
    }

    fn restore(&self, _new_db: &str) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Attempted to restore in-memory database",
        ))
    }
}

impl Encode for DB {
    fn encode_to<T: Output>(&self, dest: &mut T) {
        let columns = self.columns.read();
        let column_length = columns.len() as u32;
        column_length.encode_to(dest);
        for i in 0..column_length {
            let column = columns.get(&i).unwrap();
            column.encode_to(dest);
        }
    }
}

impl Decode for DB {
    fn decode<I: Input>(value: &mut I) -> Result<Self, Error> {
        let length = u32::decode(value)?;

        let mut db = DB::default();
        let mut map: HashMap<u32, BTreeMap<Vec<u8>, DBValue>> = HashMap::new();

        for i in 0..length {
            let v: BTreeMap<Vec<u8>, DBValue> = BTreeMap::decode(value)?;
            map.insert(i, v);
        }

        db.columns = Arc::new(RwLock::new(map));

        return Ok(db);
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use kvdb::KeyValueDB;
    use parity_scale_codec::{Decode, Encode};

    use crate::db::{create, Data, DB};
    use crate::genesis::GenesisData;

    #[test]
    fn db_encode_decode() {
        let db = create(2);
        let mut transaction = db.transaction();
        transaction.put(0, b"key1", b"horse");
        transaction.put(1, b"key2", b"pigeon");
        transaction.put(1, b"key3", b"cat");
        assert!(db.write(transaction).is_ok());

        let encoded_db = db.encode();
        assert!(encoded_db.len() > 0);
        let decoded_db = DB::decode(&mut encoded_db.as_slice()).unwrap();

        assert_eq!(decoded_db.get(0, b"key1").unwrap().unwrap(), b"horse");
        assert_eq!(decoded_db.get(1, b"key2").unwrap().unwrap(), b"pigeon");
        assert_eq!(decoded_db.get(1, b"key3").unwrap().unwrap(), b"cat");
    }

    #[test]
    fn data_encode_decode() {
        let db = create(2);
        let mut transaction = db.transaction();
        transaction.put(0, b"key1", b"horse");
        transaction.put(1, b"key2", b"pigeon");
        transaction.put(1, b"key3", b"cat");
        assert!(db.write(transaction).is_ok());

        let data = Data {
            db,
            genesis_data: GenesisData {},
        };

        let data = data.encode();
        assert!(data.len() > 0);
        let decoded_data = Data::decode(&mut data.as_slice()).unwrap();
        let decoded_db = decoded_data.db;

        assert_eq!(decoded_db.get(0, b"key1").unwrap().unwrap(), b"horse");
        assert_eq!(decoded_db.get(1, b"key2").unwrap().unwrap(), b"pigeon");
        assert_eq!(decoded_db.get(1, b"key3").unwrap().unwrap(), b"cat");
    }

    #[test]
    fn db_deterministic_encode_decode() {
        let db = create(2);
        let mut transaction = db.transaction();
        transaction.put(0, b"key1", b"horse");
        transaction.put(1, b"key2", b"pigeon");
        transaction.put(1, b"key3", b"cat");
        assert!(db.write(transaction).is_ok());

        // First test: If two DB instance are identical, their
        // deserialization need to produce same binary data.
        for _i in 0..100 {
            // Serialization
            let encoded_db = db.encode();
            assert!(encoded_db.len() > 0);
            // Deserialization
            let decoded_db = DB::decode(&mut encoded_db.as_slice()).unwrap();
            // Deserialization need to produce same data every time
            assert_eq!(encoded_db.as_slice(), decoded_db.encode().as_slice());
        }

        // Second test: If two instances of DBs are created from same binary blob,
        // and if we insert same data on both instance, then
        // both instance should produce same binary blob
        let encoded_db = db.encode();
        let decoded_db = DB::decode(&mut encoded_db.as_slice()).unwrap();

        let mut transaction = db.transaction();
        transaction.put(0, b"another_format", b"pikachu");
        let duplicate_transaction = transaction.clone();
        // Insert into original db
        assert!(db.write(transaction).is_ok());
        // Insert into an instance created from previous state of original db
        assert!(decoded_db.write(duplicate_transaction).is_ok());

        assert_eq!(db.encode().as_slice(), decoded_db.encode().as_slice());
    }

    #[test]
    fn data_deterministic_encode_decode() {
        let db = create(2);
        let mut transaction = db.transaction();
        transaction.put(0, b"key1", b"horse");
        transaction.put(1, b"key2", b"pigeon");
        transaction.put(1, b"key3", b"cat");
        assert!(db.write(transaction).is_ok());

        let data = Data {
            db,
            genesis_data: GenesisData {},
        };

        // First test: If two Data instance are identical, their
        // deserialization need to produce same binary data.
        for _i in 0..100 {
            // Serialization
            let encoded_data = data.encode();
            assert!(encoded_data.len() > 0);
            // Deserialization
            let decoded_data = Data::decode(&mut encoded_data.as_slice()).unwrap();
            // Deserialization need to produce same data every time
            assert_eq!(encoded_data.as_slice(), decoded_data.encode().as_slice());
        }

        // Second test: If two instances of DBs are created from same binary blob,
        // and if we insert same data on both instance, then
        // both instance should produce same binary blob
        let encoded_data = data.encode();
        let decoded_data = Data::decode(&mut encoded_data.as_slice()).unwrap();

        let mut transaction = data.db.transaction();
        transaction.put(0, b"another_format", b"pikachu");
        let duplicate_transaction = transaction.clone();
        // Insert into original db
        assert!(data.db.write(transaction).is_ok());

        // Insert into an instance created from previous state of original db
        assert!(decoded_data.db.write(duplicate_transaction).is_ok());

        assert_eq!(data.encode().as_slice(), decoded_data.encode().as_slice());
    }

    #[test]
    fn db_get_fails_with_non_existing_column() -> io::Result<()> {
        let db = create(1);
        assert!(db.get(1, &[]).is_err());
        Ok(())
    }

    #[test]
    fn db_put_and_get() -> io::Result<()> {
        let db = create(1);
        let key1 = b"key1";

        let mut transaction = db.transaction();
        transaction.put(0, key1, b"horse");
        db.write_buffered(transaction);
        assert_eq!(&*db.get(0, key1)?.unwrap(), b"horse");
        Ok(())
    }

    #[test]
    fn db_delete_and_get() -> io::Result<()> {
        let db = create(1);
        let key1 = b"key1";

        let mut transaction = db.transaction();
        transaction.put(0, key1, b"horse");
        db.write_buffered(transaction);
        assert_eq!(&*db.get(0, key1)?.unwrap(), b"horse");

        let mut transaction = db.transaction();
        transaction.delete(0, key1);
        db.write_buffered(transaction);
        assert!(db.get(0, key1)?.is_none());
        Ok(())
    }

    #[test]
    fn db_iter() -> io::Result<()> {
        let db = create(1);
        let key1 = b"key1";
        let key2 = b"key2";

        let mut transaction = db.transaction();
        transaction.put(0, key1, key1);
        transaction.put(0, key2, key2);
        db.write_buffered(transaction);

        let contents: Vec<_> = db.iter(0).into_iter().collect();
        assert_eq!(contents.len(), 2);
        assert_eq!(&*contents[0].0, key1);
        assert_eq!(&*contents[0].1, key1);
        assert_eq!(&*contents[1].0, key2);
        assert_eq!(&*contents[1].1, key2);
        Ok(())
    }

    #[test]
    fn db_iter_from_prefix() -> io::Result<()> {
        let db = create(1);
        let key1 = b"0";
        let key2 = b"ab";
        let key3 = b"abc";
        let key4 = b"abcd";

        let mut batch = db.transaction();
        batch.put(0, key1, key1);
        batch.put(0, key2, key2);
        batch.put(0, key3, key3);
        batch.put(0, key4, key4);
        db.write(batch)?;

        // empty prefix
        let contents: Vec<_> = db.iter_from_prefix(0, b"").into_iter().collect();
        assert_eq!(contents.len(), 4);
        assert_eq!(&*contents[0].0, key1);
        assert_eq!(&*contents[1].0, key2);
        assert_eq!(&*contents[2].0, key3);
        assert_eq!(&*contents[3].0, key4);

        // prefix a
        let contents: Vec<_> = db.iter_from_prefix(0, b"a").into_iter().collect();
        assert_eq!(contents.len(), 3);
        assert_eq!(&*contents[0].0, key2);
        assert_eq!(&*contents[1].0, key3);
        assert_eq!(&*contents[2].0, key4);

        // prefix abc
        let contents: Vec<_> = db.iter_from_prefix(0, b"abc").into_iter().collect();
        assert_eq!(contents.len(), 2);
        assert_eq!(&*contents[0].0, key3);
        assert_eq!(&*contents[1].0, key4);

        // prefix abcde
        let contents: Vec<_> = db.iter_from_prefix(0, b"abcde").into_iter().collect();
        assert_eq!(contents.len(), 0);

        // prefix 0
        let contents: Vec<_> = db.iter_from_prefix(0, b"0").into_iter().collect();
        assert_eq!(contents.len(), 1);
        assert_eq!(&*contents[0].0, key1);
        Ok(())
    }

    #[test]
    fn db_complex() -> io::Result<()> {
        let db = create(1);
        let key1 = b"02c69be41d0b7e40352fc85be1cd65eb03d40ef8427a0ca4596b1ead9a00e9fc";
        let key2 = b"03c69be41d0b7e40352fc85be1cd65eb03d40ef8427a0ca4596b1ead9a00e9fc";
        let key3 = b"04c00000000b7e40352fc85be1cd65eb03d40ef8427a0ca4596b1ead9a00e9fc";
        let key4 = b"04c01111110b7e40352fc85be1cd65eb03d40ef8427a0ca4596b1ead9a00e9fc";
        let key5 = b"04c02222220b7e40352fc85be1cd65eb03d40ef8427a0ca4596b1ead9a00e9fc";

        let mut batch = db.transaction();
        batch.put(0, key1, b"cat");
        batch.put(0, key2, b"dog");
        batch.put(0, key3, b"caterpillar");
        batch.put(0, key4, b"beef");
        batch.put(0, key5, b"fish");
        db.write(batch)?;

        assert_eq!(&*db.get(0, key1)?.unwrap(), b"cat");

        let contents: Vec<_> = db.iter(0).into_iter().collect();
        assert_eq!(contents.len(), 5);
        assert_eq!(contents[0].0.to_vec(), key1.to_vec());
        assert_eq!(&*contents[0].1, b"cat");
        assert_eq!(contents[1].0.to_vec(), key2.to_vec());
        assert_eq!(&*contents[1].1, b"dog");

        let mut prefix_iter = db.iter_from_prefix(0, b"04c0");
        assert_eq!(*prefix_iter.next().unwrap().1, b"caterpillar"[..]);
        assert_eq!(*prefix_iter.next().unwrap().1, b"beef"[..]);
        assert_eq!(*prefix_iter.next().unwrap().1, b"fish"[..]);

        let mut batch = db.transaction();
        batch.delete(0, key1);
        db.write(batch)?;

        assert!(db.get(0, key1)?.is_none());

        let mut batch = db.transaction();
        batch.put(0, key1, b"cat");
        db.write(batch)?;

        let mut transaction = db.transaction();
        transaction.put(0, key3, b"elephant");
        transaction.delete(0, key1);
        db.write(transaction)?;
        assert!(db.get(0, key1)?.is_none());
        assert_eq!(&*db.get(0, key3)?.unwrap(), b"elephant");

        assert_eq!(&*db.get_by_prefix(0, key3).unwrap(), b"elephant");
        assert_eq!(&*db.get_by_prefix(0, key2).unwrap(), b"dog");

        let mut transaction = db.transaction();
        transaction.put(0, key1, b"horse");
        transaction.delete(0, key3);
        db.write_buffered(transaction);
        assert!(db.get(0, key3)?.is_none());
        assert_eq!(&*db.get(0, key1)?.unwrap(), b"horse");

        db.flush()?;
        assert!(db.get(0, key3)?.is_none());
        assert_eq!(&*db.get(0, key1)?.unwrap(), b"horse");
        Ok(())
    }
}

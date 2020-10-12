pub trait PrimaryKey<'a> {
    type Prefix: Prefixer<'a>;

    /// returns a slice of key steps, which can be optionally combined
    fn key<'b>(&'b self) -> Vec<&'b [u8]>
    where
        'a: 'b;
}

impl<'a> PrimaryKey<'a> for &'a [u8] {
    type Prefix = ();

    fn key<'b>(&'b self) -> Vec<&'b [u8]>
    where
        'a: 'b,
    {
        // this is simple, we don't add more prefixes
        vec![self]
    }
}

impl<'a> PrimaryKey<'a> for (&'a [u8], &'a [u8]) {
    type Prefix = &'a [u8];

    fn key<'b>(&'b self) -> Vec<&'b [u8]>
    where
        'a: 'b,
    {
        vec![self.0, self.1]
    }
}

impl<'a> PrimaryKey<'a> for (&'a [u8], &'a [u8], &'a [u8]) {
    type Prefix = (&'a [u8], &'a [u8]);

    fn key<'b>(&'b self) -> Vec<&'b [u8]>
    where
        'a: 'b,
    {
        vec![self.0, self.1, self.2]
    }
}

pub trait Prefixer<'a> {
    /// returns 0 or more namespaces that should length-prefixed and concatenated for range searches
    fn prefix(&self) -> Vec<&'a [u8]>;
}

impl<'a> Prefixer<'a> for () {
    fn prefix(&self) -> Vec<&'a [u8]> {
        vec![]
    }
}

impl<'a> Prefixer<'a> for &'a [u8] {
    fn prefix(&self) -> Vec<&'a [u8]> {
        vec![self]
    }
}

impl<'a> Prefixer<'a> for (&'a [u8], &'a [u8]) {
    fn prefix(&self) -> Vec<&'a [u8]> {
        vec![self.0, self.1]
    }
}

// Add support for an dynamic keys - constructor functions below
pub struct Pk1Owned(pub Vec<u8>);

pub fn u64_key(val: u64) -> Pk1Owned {
    Pk1Owned(val.to_be_bytes().into())
}

impl<'a> PrimaryKey<'a> for Pk1Owned {
    type Prefix = ();

    fn key<'b>(&'b self) -> Vec<&'b [u8]>
    where
        'a: 'b,
    {
        vec![&self.0]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn u64key_works() {
        let k = u64_key(134);
        let path = k.key();
        assert_eq!(1, path.len());
        assert_eq!(134u64.to_be_bytes().to_vec(), path[0].to_vec());
    }
}

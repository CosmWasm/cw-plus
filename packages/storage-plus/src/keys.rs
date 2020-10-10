pub trait PrimaryKey<'a> {
    type Prefix: Prefixer<'a>;

    /// returns a slice of key steps, which can be optionally combined
    fn key(&self) -> Vec<&'a [u8]>;
}

impl<'a> PrimaryKey<'a> for &'a [u8] {
    type Prefix = ();

    fn key(&self) -> Vec<&'a [u8]> {
        // this is simple, we don't add more prefixes
        vec![self]
    }
}

impl<'a> PrimaryKey<'a> for (&'a [u8], &'a [u8]) {
    type Prefix = &'a [u8];

    fn key(&self) -> Vec<&'a [u8]> {
        vec![self.0, self.1]
    }
}

impl<'a> PrimaryKey<'a> for (&'a [u8], &'a [u8], &'a [u8]) {
    type Prefix = (&'a [u8], &'a [u8]);

    fn key(&self) -> Vec<&'a [u8]> {
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

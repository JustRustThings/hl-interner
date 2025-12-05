#![warn(unused_crate_dependencies)]

use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

pub static STR_INTERNER: StrInterner = StrInterner::new();

pub struct StrInterner {
    pool: scc::HashSet<SharedStr, foldhash::fast::FixedState>,
}

impl StrInterner {
    const fn new() -> Self {
        Self {
            pool: scc::HashSet::with_hasher(foldhash::fast::FixedState::with_seed(0)),
        }
    }

    pub fn get<S: AsStr>(&self, text: S) -> SharedStr {
        if let Some(s) = self.pool.read_sync(text.as_ref(), |s| s.clone()) {
            return s;
        }
        let val = text.to_owned();
        let val = val.into_boxed_str();
        let s = SharedStr(Arc::new(val));
        let _res = self.pool.insert_sync(s.clone());
        s
    }
}

#[derive(Clone, Default)]
pub struct SharedStr(Arc<Box<str>>);

impl SharedStr {
    pub fn as_str(&self) -> &str {
        self.0.deref()
    }
}

impl std::ops::Deref for SharedStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Drop for SharedStr {
    fn drop(&mut self) {
        // this one + the one in the interner => this is the last usage
        if Arc::strong_count(&self.0) == 2 {
            // no atomic sync => at worst a shared string will be detached from the pool
            STR_INTERNER.pool.remove_sync(self);
        }
    }
}

impl std::fmt::Debug for SharedStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.deref().fmt(f)
    }
}

impl std::fmt::Display for SharedStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.deref().fmt(f)
    }
}

impl Hash for SharedStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.deref().hash(state);
    }
}

impl PartialEq for SharedStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.deref().eq(other.0.deref())
    }
}

impl Eq for SharedStr {}

impl scc::Equivalent<SharedStr> for str {
    fn equivalent(&self, key: &SharedStr) -> bool {
        self == key.0.deref().deref()
    }
}

pub trait AsStr: AsRef<str> {
    fn to_owned(self) -> String;
}

// yep, `ToOwned<Owned=String>` is implemented for `str` not for `&str`
impl AsStr for &str {
    fn to_owned(self) -> String {
        self.to_string()
    }
}

impl AsStr for String {
    fn to_owned(self) -> String {
        self
    }
}

impl From<&str> for SharedStr {
    fn from(value: &str) -> Self {
        STR_INTERNER.get(value)
    }
}

impl From<String> for SharedStr {
    fn from(value: String) -> Self {
        STR_INTERNER.get(value)
    }
}

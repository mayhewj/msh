use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::Path;

use crate::ast::{Exportable, NameValuePair};
use crate::Result;

pub struct Environment {
    values: HashMap<OsString, Var>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: env::vars_os()
                .map(|(name, value)| (name, Var::new(value, true)))
                .collect(),
        }
    }

    pub fn get<N: AsRef<OsStr>>(&self, name: N) -> Option<&OsStr> {
        self.values.get(name.as_ref()).map(|var| var.value.as_ref())
    }

    pub fn assign(&mut self, pair: &NameValuePair) -> Result<()> {
        let value = pair.value.expand(self)?.into_owned();
        match self.values.entry(pair.name.to_os_string()) {
            Entry::Occupied(mut entry) => entry.get_mut().value = value,
            Entry::Vacant(entry) => {
                entry.insert(Var::new(value, false));
            }
        }
        Ok(())
    }

    pub fn export(&mut self, exportable: &Exportable) -> Result<()> {
        if let Some(ref value) = exportable.value {
            let var = Var::new(value.expand(self)?.into_owned(), true);
            self.values.insert(exportable.name.to_os_string(), var);
        } else {
            match self.values.entry(exportable.name.to_os_string()) {
                Entry::Occupied(mut entry) => entry.get_mut().is_exported = true,
                Entry::Vacant(entry) => {
                    entry.insert(Var::new(OsString::from(""), true));
                }
            }
        }
        Ok(())
    }

    pub fn home(&self) -> &Path {
        Path::new(self.get("HOME").expect("HOME required"))
    }

    pub fn path(&self) -> &OsStr {
        match self.get("PATH") {
            Some(value) => value,
            None => OsStr::new(""),
        }
    }

    pub fn iter_exported(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        self.values
            .iter()
            .filter(|(_, var)| var.is_exported)
            .map(|(name, var)| (name.as_os_str(), var.value.as_os_str()))
    }
}

struct Var {
    value: OsString,
    is_exported: bool,
}

impl Var {
    fn new(value: OsString, is_exported: bool) -> Self {
        Self { value, is_exported }
    }
}

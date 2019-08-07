use rusqlite::{Connection, NO_PARAMS};

use crate::model::{Backend,Dictionary,Entry,JsonEntry,PartOfSpeech,BackendError};

pub struct RusqliteState {
    path : String
}

impl Backend for RusqliteState {
    /// List the identifiers for all dictionaries
    fn dictionaries(&self) -> Result<Vec<String>,BackendError> {
        let db = Connection::open(&self.path)?;
        let mut stmt = db.prepare("SELECT id FROM dictionaries")?;
        let mut result = stmt.query(NO_PARAMS)?;
        let mut dict_list = Vec::new();
        while let Some(r) = result.next()? {
            dict_list.push(r.get(0)?);
        }
        Ok(dict_list)
    }
    /// Obtain the metadata about a given dictionary
    fn about(&self, dictionary : &str) -> Result<Dictionary,BackendError> {
        panic!("TODO")
    }
    /// List all entries in a dictrionary
    fn list(&self, dictionary : &str, offset : Option<usize>, 
            limit : Option<usize>) -> Result<Vec<Entry>,BackendError> {
        panic!("TODO")
    }
    /// Search the dictionary by headword
    fn lookup(&self, dictionary : &str, headword : &str,
              offset : Option<usize>, limit : Option<usize>,
              part_of_speech : Option<PartOfSpeech>, inflected : bool) -> Result<Vec<Entry>,BackendError> {
        panic!("TODO")
    }
    /// Get the content as Json
    fn entry_json(&self, dictionary : &str, id : &str) -> Result<JsonEntry,BackendError> {
        panic!("TODO")
    }
    /// Get the content as OntoLex
    fn entry_ontolex(&self, dictionary : &str, id : &str) -> Result<String,BackendError> {
        panic!("TODO")
    }
    /// Get the content as TEI
    fn entry_tei(&self, dictionary : &str, id : &str) -> Result<String,BackendError> {
        panic!("TODO")
    }

}


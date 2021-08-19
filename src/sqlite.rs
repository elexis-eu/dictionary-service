use rusqlite::{Connection};

use crate::model::{Backend,Dictionary,Entry,JsonEntry,PartOfSpeech,BackendError,Release,EntryContent,Format};
#[cfg(test)]
use crate::model::{Agent,Genre};
use std::collections::HashMap;
#[cfg(test)]
use std::fs;

#[derive(Clone,StateData)]
pub struct RusqliteState {
    path : String
}

impl RusqliteState {
    pub fn new(path : &str) -> Self {
        RusqliteState { path:path.to_string() }
    }

    pub fn load(&self,
        release : Release,
        dictionaries : HashMap<String, Dictionary>,
        dict_entries : HashMap<String, Vec<EntryContent>>) -> Result<(),rusqlite::Error> {
        let db = Connection::open(&self.path)?;
        self.create_tables(&db)?;
        for (dict_id, dict) in dictionaries {
            self.insert_dict(&db, &dict_id, dict)?;
        }
        for (dict_id, entries) in dict_entries {
            for entry in entries {
                self.insert_entry(&db, &dict_id, entry, release.clone())?;
            }
        }
        Ok(())

    }

    fn create_tables(&self, db : &rusqlite::Connection) -> Result<(),rusqlite::Error> {
        db.execute("CREATE TABLE IF NOT EXISTS dictionaries
                (id TEXT UNIQUE,
                 release TEXT,
                 source_language TEXT,
                 target_languages TEXT,
                 genres TEXT,
                 license TEXT,
                 creators TEXT,
                 publishers TEXT)", [])?;
        db.execute("CREATE TABLE IF NOT EXISTS dictionary_dc
                (id TEXT,
                 prop TEXT,
                 value TEXT)", [])?;
        db.execute("CREATE INDEX IF NOT EXISTS dictionary_dc_idx ON dictionary_dc (id)", [])?;
        db.execute("CREATE TABLE IF NOT EXISTS entries
                (row_id INTEGER PRIMARY KEY,
                 release TEXT,
                 lemma TEXT,
                 id TEXT,
                 part_of_speech TEXT,
                 dict TEXT,
                 UNIQUE(dict,id))", [])?;
        db.execute("CREATE INDEX IF NOT EXISTS entries_idx ON entries (lemma)", [])?;
        db.execute("CREATE INDEX IF NOT EXISTS entries_idx2 ON entries (dict)", [])?;
        db.execute("CREATE INDEX IF NOT EXISTS entries_idx3 ON entries (id)", [])?;
        db.execute("CREATE TABLE IF NOT EXISTS variants
                (entry_id INTEGER,
                 form TEXT,
                 FOREIGN KEY (entry_id) REFERENCES entries(row_id))", [])?;
        db.execute("CREATE TABLE IF NOT EXISTS json_entries
                (entry_id INTEGER,
                 json TEXT,
                 FOREIGN KEY (entry_id) REFERENCES entries(row_id))", [])?;
        db.execute("CREATE INDEX IF NOT EXISTS json_entries_idx ON json_entries (entry_id)", [])?;
        db.execute("CREATE TABLE IF NOT EXISTS ontolex_entries
                (entry_id INTEGER,
                 ontolex TEXT,
                 FOREIGN KEY (entry_id) REFERENCES entries(row_id))", [])?;
        db.execute("CREATE INDEX IF NOT EXISTS ontolex_entries_idx ON ontolex_entries (entry_id)", [])?;
        db.execute("CREATE TABLE IF NOT EXISTS tei_entries
                (entry_id INTEGER,
                 tei TEXT,
                 FOREIGN KEY (entry_id) REFERENCES entries(row_id))", [])?;
        db.execute("CREATE INDEX IF NOT EXISTS tei_entries_idx ON tei_entries (entry_id)", [])?;
        Ok(())
    }

    fn insert_dict(&self, db : &Connection, dict_id : &str, dict : Dictionary) -> Result<(),rusqlite::Error> {
        let mut stmt = db.prepare("INSERT OR REPLACE INTO dictionaries (id, release, source_language, target_languages, genres, license, creators, publishers) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")?;

        stmt.execute(
            &[dict_id, &serde_json::to_string(&dict.release).unwrap(), 
              &dict.source_language,
              &serde_json::to_string(&dict.target_language).unwrap(),
              &serde_json::to_string(&dict.genre).unwrap(),
              &dict.license,
              &serde_json::to_string(&dict.creator).unwrap(),
              &serde_json::to_string(&dict.publisher).unwrap()])?;

        let mut stmt = db.prepare("INSERT INTO dictionary_dc (id, prop, value) VALUES (?,?,?)")?;
        for (prop, value) in dict.get_dc_props().iter() {
            stmt.execute(&[dict_id, *prop, value])?;
        }
        Ok(())
    }
    fn insert_entry(&self, db : &Connection, dict_id : &str, entry_content : EntryContent, release : Release) -> Result<(),rusqlite::Error> {
        let row_id : u32 = if let Ok(r) = db.query_row("SELECT row_id FROM entries WHERE id=? AND dict=?", &[entry_content.id(), dict_id], |r| r.get(0)) {
            r
        } else {

            let mut stmt = db.prepare("INSERT OR REPLACE INTO entries (release, lemma, id, part_of_speech, dict) VALUES (?,?,?,?,?)")?;
            stmt.execute(&[
                &serde_json::to_string(&release).unwrap(),
                entry_content.lemma(),
                entry_content.id(),
                &serde_json::to_string(&entry_content.pos()).unwrap(),
                dict_id])?;

            let mut stmt2 = db.prepare("SELECT last_insert_rowid()")?;
            let mut result = stmt2.query([])?;

            if let Some(r) = result.next()? {
                r.get(0)? 
            } else {
                panic!("After INSERT did not return for last_insert_rowid()")    
            }
        };

        let mut stmt3 = db.prepare("INSERT INTO variants (entry_id, form) VALUES (?,?)")?;
        stmt3.execute(&[&format!("{}",row_id), entry_content.lemma()])?;
        for v in entry_content.variants() {
            stmt3.execute(&[&format!("{}",row_id), &v])?;
        }

        match entry_content {
            EntryContent::Json(_) => {
                let mut stmt4 = db.prepare("INSERT INTO json_entries (entry_id, json) VALUES(?,?)")?;
                stmt4.execute(&[&format!("{}",row_id), &entry_content.content()])?;
            }
            EntryContent::Tei(_,_,_,_,_) => {
                let mut stmt4 = db.prepare("INSERT INTO tei_entries (entry_id, tei) VALUES(?,?)")?;
                stmt4.execute(&[&format!("{}",row_id), &entry_content.content()])?;
            }
            EntryContent::OntoLex(_,_,_,_,_) => {
                let mut stmt4 = db.prepare("INSERT INTO ontolex_entries (entry_id, ontolex) VALUES(?,?)")?;
                stmt4.execute(&[&format!("{}",row_id), &entry_content.content()])?;
            }
        }


        Ok(())

    }

    pub fn delete(&self, dict_id : &str) -> Result<(),BackendError> {
        let db = Connection::open(&self.path)?;
        db.execute("DELETE FROM dictionaries WHERE id=?", &[dict_id])?;
        db.execute("DELETE FROM dictionary_dc WHERE id=?", &[dict_id])?;
        db.execute("DELETE FROM variants WHERE entry_id IN (SELECT row_id FROM entries WHERE dict=?)", &[dict_id])?;
        db.execute("DELETE FROM json_entries WHERE entry_id IN (SELECT row_id FROM entries WHERE dict=?)", &[dict_id])?;
        db.execute("DELETE FROM tei_entries WHERE entry_id IN (SELECT row_id FROM entries WHERE dict=?)", &[dict_id])?;
        db.execute("DELETE FROM ontolex_entries WHERE entry_id IN (SELECT row_id FROM entries WHERE dict=?)", &[dict_id])?;
        db.execute("DELETE FROM entries WHERE dict=?", &[dict_id])?;
        
        Ok(())
    }
}

impl Backend for RusqliteState {
    /// List the identifiers for all dictionaries
    fn dictionaries(&self) -> Result<Vec<String>,BackendError> {
        let db = Connection::open(&self.path)?;
        let mut stmt = db.prepare("SELECT id FROM dictionaries")?;
        let mut result = stmt.query([])?;
        let mut dict_list = Vec::new();
        while let Some(r) = result.next()? {
            dict_list.push(r.get(0)?);
        }
        Ok(dict_list)
    }
    /// Obtain the metadata about a given dictionary
    fn about(&self, dictionary : &str) -> Result<Dictionary,BackendError> {
        let db = Connection::open(&self.path)?;
        let mut stmt = db.prepare("SELECT release, source_language, target_languages, genres, license, creators, publishers FROM dictionaries WHERE id=?")?;
        let mut result = stmt.query(&[dictionary])?;
        
        if let Some(r) = result.next()? {
            let r_str : String = r.get(0)?;
            let release = serde_json::from_str(&r_str)?;
            let source_lang = r.get(1)?;
            let tl_str : String = r.get(2)?;
            let targ_langs = serde_json::from_str(&tl_str)?;
            let g_str : String = r.get(3)?;
            let genres = serde_json::from_str(&g_str)?;
            let license = r.get(4)?;
            let c_str : String = r.get(5)?;
            let creators = serde_json::from_str(&c_str)?;
            let p_str : String = r.get(6)?;
            let publishers = serde_json::from_str(&p_str)?;

            let mut dict = Dictionary::new(release, source_lang, targ_langs,
                    genres, license, creators, publishers);

            let mut stmt = db.prepare("SELECT prop, value FROM dictionary_dc WHERE id=?")?;
            let rows = stmt.query_map(&[dictionary], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;

            for row in rows {
                let (prop, value) : (String, String) = row?;
                dict.set_dc_prop(&prop, &value);
            }

            Ok(dict)

        } else {
            Err(BackendError::NotFound)
        }
    }
    /// List all entries in a dictrionary
    fn list(&self, dictionary : &str, offset : Option<usize>, 
            limit : Option<usize>) -> Result<Vec<Entry>,BackendError> {
        let db = Connection::open(&self.path)?;
        let mut stmt = match offset {
            Some(_) => match limit {
                Some(_) =>
                    db.prepare("SELECT release, lemma, id, part_of_speech, row_id FROM entries WHERE dict=? LIMIT ? OFFSET ?")?,
                None =>
                    db.prepare("SELECT release, lemma, id, part_of_speech, row_id FROM entries WHERE dict=? LIMIT -1 OFFSET ?")?
            },
            None => match limit {
                Some(_) =>
                    db.prepare("SELECT release, lemma, id, part_of_speech, row_id FROM entries WHERE dict=? LIMIT ?")?,
                None =>
                    db.prepare("SELECT release, lemma, id, part_of_speech, row_id FROM entries WHERE dict=?")?
            }
        };
        let mut result = match offset {
            Some(o) => match limit {
                Some(l) => {
                    let lstr = format!("{}",l);
                    let ostr = format!("{}",o);
                    stmt.query(&[dictionary, &lstr, &ostr])?
                },
                None => {
                    let ostr = format!("{}",o);
                    stmt.query(&[dictionary, &ostr])?
                }
            },
            None => match limit {
                Some(l) => {
                    let lstr = format!("{}",l);
                    stmt.query(&[dictionary, &lstr])?
                },
                None =>
                    stmt.query(&[dictionary])?
            }
        };

        let mut entries = Vec::new();

        while let Some(r) = result.next()? {
            let r_str : String = r.get(0)?;
            let pos_str : String = r.get(3)?;
            let row_id : u32 = r.get(4)?;
            entries.push(Entry {
                release: serde_json::from_str(&r_str)?,
                lemma: r.get(1)?,
                id: r.get(2)?,
                part_of_speech: serde_json::from_str(&pos_str)?,
                formats: extract_formats(row_id, &db)
            })
        }

        if entries.is_empty() {
            if let Ok(_) = db.query_row("SELECT * FROM dictionaries WHERE id=?", &[dictionary], |_| Ok(())) {
                Ok(entries)
            } else {
                Err(BackendError::NotFound)
            }
        } else {
            Ok(entries)
        }
    }

    /// Search the dictionary by headword
    fn lookup(&self, dictionary : &str, headword : &str,
              offset : Option<usize>, limit : Option<usize>,
              part_of_speech : Option<PartOfSpeech>, inflected : bool) -> Result<Vec<Entry>,BackendError> {
        let db = Connection::open(&self.path)?;
        let mut q = String::from("SELECT release, lemma, id, part_of_speech, row_id FROM entries");
        
        if inflected {
            q.push_str(" JOIN variants ON variants.entry_id == entries.row_id WHERE dict=?");
        } else {
            q.push_str(" WHERE dict=?");
        }
        let mut params = Vec::new();
        params.push(dictionary);
        let mut pos_str = String::new();
        if let Some(pos) = part_of_speech {
            q.push_str(" AND part_of_speech=?");
            pos_str.push_str(&format!("{:?}", pos));
            params.push(&pos_str);
        }
        if inflected {
            q.push_str(" AND variants.form=?");
        } else {
            q.push_str(" AND entries.lemma=?");
        }
        params.push(headword);
        let mut o_str = String::new();
        let mut l_str = String::new();
        if let Some(l) = limit {
            q.push_str(" LIMIT ?");
            l_str.push_str(&format!("{}", l));
            params.push(&l_str);
        } else {
            q.push_str(" LIMIT -1");
        }

        if let Some(o) = offset {
            q.push_str(" OFFSET ?");
            o_str.push_str(&format!("{}", o));
            params.push(&o_str);
        }
        let mut stmt = db.prepare(&q)?;
        let mut result = stmt.query(rusqlite::params_from_iter(params.iter()))?;
        let mut entries = Vec::new();
        while let Some(r) = result.next()? {
            let r_str : String = r.get(0)?;
            let pos_str : String = r.get(3)?;
            let row_id : u32 = r.get(4)?;
            entries.push(Entry {
                release: serde_json::from_str(&r_str)?,
                lemma: r.get(1)?,
                id: r.get(2)?,
                part_of_speech: serde_json::from_str(&pos_str)?,
                formats: extract_formats(row_id, &db)
            })
        }
 
        if entries.is_empty() {
            if let Ok(_) = db.query_row("SELECT * FROM dictionaries WHERE id=?", &[dictionary], |_| Ok(())) {
                Ok(entries)
            } else {
                Err(BackendError::NotFound)
            }
        } else {
            Ok(entries)
        }

    }
    /// Get the content as Json
    fn entry_json(&self, dictionary : &str, id : &str) -> Result<JsonEntry,BackendError> {
        let db = Connection::open(&self.path)?;
        let mut stmt = db.prepare("SELECT json FROM json_entries JOIN entries ON entries.row_id == json_entries.entry_id WHERE dict=? AND id=?")?;
        let mut result = stmt.query(&[dictionary, id])?;
        if let Some(r) = result.next()? {
            let json_str : String = r.get(0)?;
            Ok(serde_json::from_str(&json_str)?)
        } else {
            Err(BackendError::NotFound)
        }
    }
    /// Get the content as OntoLex
    fn entry_ontolex(&self, dictionary : &str, id : &str) -> Result<String,BackendError> {
        let db = Connection::open(&self.path)?;
        let mut stmt = db.prepare("SELECT ontolex FROM ontolex_entries JOIN entries ON entries.row_id == ontolex_entries.entry_id WHERE dict=? AND id=?")?;
        let mut result = stmt.query(&[dictionary, id])?;
        if let Some(r) = result.next()? {
            Ok(r.get(0)?)
        } else {
            Err(BackendError::NotFound)
        }
    }
    /// Get the content as TEI
    fn entry_tei(&self, dictionary : &str, id : &str) -> Result<String,BackendError> {
        let db = Connection::open(&self.path)?;
        let mut stmt = db.prepare("SELECT tei FROM tei_entries JOIN entries ON entries.row_id == tei_entries.entry_id WHERE dict=? AND id=?")?;
        let mut result = stmt.query(&[dictionary, id])?;
        if let Some(r) = result.next()? {
            Ok(r.get(0)?)
        } else {
            Err(BackendError::NotFound)
        }
     }

}

fn extract_formats(row_id : u32, connection : &Connection) -> Vec<Format> {
    let mut formats = Vec::new();
    if connection.query_row("SELECT * FROM json_entries WHERE entry_id=?", &[&row_id], |_| Ok(())).is_ok() {
        formats.push(Format::json);
    }
    if connection.query_row("SELECT * FROM tei_entries WHERE entry_id=?", &[&row_id], |_| Ok(())).is_ok() {
        formats.push(Format::tei);
    }
    if connection.query_row("SELECT * FROM ontolex_entries WHERE entry_id=?", &[&row_id], |_| Ok(())).is_ok() {
        formats.push(Format::ontolex);
    }

    formats
}


#[test]
fn test_create_db() {
    let state = RusqliteState::new("test-tmp.db");
    state.load(Release::PUBLIC, HashMap::new(), HashMap::new()).unwrap();
    fs::remove_file("test-tmp.db").unwrap();
}

#[test]
fn test_load_db() {
    let state = RusqliteState::new("test-tmp2.db");
    let mut dictionaries = HashMap::new();
    dictionaries.insert("dict1".to_string(),
        Dictionary::new(
            Release::PUBLIC,
            "en".to_string(),
            vec!["en".to_string(),"de".to_string()],
            vec![Genre::gen],
            "http://license.url/".to_string(),
            vec![Agent { 
                name: "Joe Bloggs".to_string(), 
                email: Some("joe@example.com".to_string()),
                url: None }],
            Vec::new()
        ));
    let mut entries = HashMap::new();
    entries.insert("dict1".to_string(), vec![
        EntryContent::Json(serde_json::from_str("{
            \"@context\": \"http://lexinfo.net/jsonld/3.0/content.json\",
            \"@type\": \"Word\",
            \"@id\": \"test\",
            \"language\": \"en\",
            \"partOfSpeech\": \"adjective\",
            \"canonicalForm\": {
                \"writtenRep\": \"example\"
            },
            \"senses\": [
                {
                    \"definition\": \"An example OntoLex Entry\"
                }
            ]
        }").unwrap())]);


    state.load(Release::PUBLIC, dictionaries, entries).unwrap();
    fs::remove_file("test-tmp2.db").unwrap();
}

#[test]
fn test_backend() {
    let state = RusqliteState::new("test-tmp3.db");
    let mut dictionaries = HashMap::new();
    dictionaries.insert("dict1".to_string(),
        Dictionary::new(
            Release::PUBLIC,
            "en".to_string(),
            vec!["en".to_string(),"de".to_string()],
            vec![Genre::gen],
            "http://license.url/".to_string(),
            vec![Agent { 
                name: "Joe Bloggs".to_string(), 
                email: Some("joe@example.com".to_string()),
                url: None }],
            Vec::new()
        ));
    let mut entries = HashMap::new();
    entries.insert("dict1".to_string(), vec![
        EntryContent::Json(serde_json::from_str("{
            \"@context\": \"http://lexinfo.net/jsonld/3.0/content.json\",
            \"@type\": \"Word\",
            \"@id\": \"test\",
            \"language\": \"en\",
            \"partOfSpeech\": \"adjective\",
            \"canonicalForm\": {
                \"writtenRep\": \"example\"
            },
            \"senses\": [
                {
                    \"definition\": \"An example OntoLex Entry\"
                }
            ]
        }").unwrap())]);


    state.load(Release::PUBLIC, dictionaries, entries).unwrap();

    let dictionaries = state.dictionaries();
    assert!(dictionaries.is_ok());
    assert_eq!(dictionaries.unwrap().len(), 1);

    let meta = state.about("dict1").unwrap();
    assert_eq!(meta.release, Release::PUBLIC);
    assert_eq!(meta.source_language, "en");
    assert_eq!(meta.target_language, vec!["en".to_string(),"de".to_string()]);
    assert_eq!(meta.genre, vec![Genre::gen]);
    assert_eq!(meta.license, "http://license.url/".to_string());
    assert_eq!(meta.creator, vec![Agent { 
        name: "Joe Bloggs".to_string(), 
        email: Some("joe@example.com".to_string()),
        url: None }]);
    assert_eq!(meta.publisher, Vec::new());

    let list = state.list("dict1",None,None).unwrap();
    assert_eq!(list.len(), 1);
    let list = state.list("dict1",Some(0),None).unwrap();
    assert_eq!(list.len(), 1);
    let list = state.list("dict1",None,Some(1)).unwrap();
    assert_eq!(list.len(), 1);
    let list = state.list("dict1",Some(0),Some(1)).unwrap();
    assert_eq!(list.len(), 1);

  
    let _lookup = state.lookup("dict1", "example", None, None, None, false).unwrap();
    let _lookup = state.lookup("dict1", "example", Some(0), None, None, false).unwrap();
    let _lookup = state.lookup("dict1", "example", None, Some(1), None, false).unwrap();
    let _lookup = state.lookup("dict1", "example", Some(0), Some(1), None, false).unwrap();
    let _lookup = state.lookup("dict1", "example", None, None, Some(PartOfSpeech::ADJ), false).unwrap();
    let _lookup = state.lookup("dict1", "example", Some(0), None, Some(PartOfSpeech::ADJ), false).unwrap();
    let _lookup = state.lookup("dict1", "example", None, Some(1), Some(PartOfSpeech::ADJ), false).unwrap();
    let _lookup = state.lookup("dict1", "example", Some(0), Some(1), Some(PartOfSpeech::ADJ), false).unwrap();
    let _lookup = state.lookup("dict1", "example", None, None, None, true).unwrap();
    let _lookup = state.lookup("dict1", "example", Some(0), None, None, true).unwrap();
    let _lookup = state.lookup("dict1", "example", None, Some(1), None, true).unwrap();
    let _lookup = state.lookup("dict1", "example", Some(0), Some(1), None, true).unwrap();
    let _lookup = state.lookup("dict1", "example", None, None, Some(PartOfSpeech::ADJ), true).unwrap();
    let _lookup = state.lookup("dict1", "example", Some(0), None, Some(PartOfSpeech::ADJ), true).unwrap();
    let _lookup = state.lookup("dict1", "example", None, Some(1), Some(PartOfSpeech::ADJ), true).unwrap();
    let _lookup = state.lookup("dict1", "example", Some(0), Some(1), Some(PartOfSpeech::ADJ), true).unwrap();

    let _entry_json = state.entry_json("dict1", "test").unwrap();
    state.entry_ontolex("dict1","test").err().unwrap();
    state.entry_tei("dict1","test").err().unwrap();
    fs::remove_file("test-tmp3.db").unwrap();
}



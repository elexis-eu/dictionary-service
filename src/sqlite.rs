use rusqlite::{Connection, NO_PARAMS};

use crate::model::{Backend,Dictionary,Entry,JsonEntry,PartOfSpeech,BackendError,Release,Genre,Agent,Format,EntryContent,entry_from_content};
use std::str::FromStr;
use std::collections::HashMap;
use std::fs;

pub struct RusqliteState {
    path : String
}

impl RusqliteState {
    pub fn new(path : String) -> Self {
        RusqliteState { path }
    }

    pub fn load(&self,
        release : Release,
        dictionaries : HashMap<String, Dictionary>,
        dict_entries : HashMap<String, Vec<EntryContent>>) -> Result<(),rusqlite::Error> {
        let db = Connection::open(&self.path)?;
        self.create_tables(&db)?;
        for (dict_id, dict) in dictionaries {
            self.insert_dict(&db, &dict_id, dict);
        }
        for (dict_id, entries) in dict_entries {
            for entry in entries {
                self.insert_entry(&db, &dict_id, entry, release.clone());
            }
        }
        Ok(())

    }

    fn create_tables(&self, db : &rusqlite::Connection) -> Result<(),rusqlite::Error> {
        db.execute("CREATE TABLE IF NOT EXISTS dictionaries
                (id TEXT,
                 release TEXT,
                 source_language TEXT,
                 target_languages TEXT,
                 genres TEXT,
                 license TEXT,
                 creators TEXT,
                 publishers TEXT)", NO_PARAMS)?;
        db.execute("CREATE TABLE IF NOT EXISTS entries
                (row_id INTEGER PRIMARY KEY,
                 release TEXT,
                 lemma TEXT,
                 id TEXT,
                 part_of_speech TEXT,
                 format TEXT,
                 dict TEXT)", NO_PARAMS)?;
        db.execute("CREATE INDEX IF NOT EXISTS entries_idx ON entries (lemma)", NO_PARAMS)?;
        db.execute("CREATE INDEX IF NOT EXISTS entries_idx2 ON entries (dict)", NO_PARAMS)?;
        db.execute("CREATE INDEX IF NOT EXISTS entries_idx3 ON entries (id)", NO_PARAMS)?;
        db.execute("CREATE TABLE IF NOT EXISTS variants
                (entry_id INTEGER,
                 form TEXT,
                 FOREIGN KEY (entry_id) REFERENCES entries(row_id))", NO_PARAMS)?;
        db.execute("CREATE TABLE IF NOT EXISTS json_entries
                (entry_id INTEGER,
                 json TEXT,
                 FOREIGN KEY (entry_id) REFERENCES entries(row_id))", NO_PARAMS)?;
        db.execute("CREATE INDEX IF NOT EXISTS json_entries_idx ON json_entries (entry_id)", NO_PARAMS)?;
        db.execute("CREATE TABLE IF NOT EXISTS ontolex_entries
                (entry_id INTEGER,
                 ontolex TEXT,
                 FOREIGN KEY (entry_id) REFERENCES entries(row_id))", NO_PARAMS)?;
        db.execute("CREATE INDEX IF NOT EXISTS ontolex_entries_idx ON ontolex_entries (entry_id)", NO_PARAMS)?;
        db.execute("CREATE TABLE IF NOT EXISTS tei_entries
                (entry_id INTEGER,
                 tei TEXT,
                 FOREIGN KEY (entry_id) REFERENCES entries(row_id))", NO_PARAMS)?;
        db.execute("CREATE INDEX IF NOT EXISTS tei_entries_idx ON tei_entries (entry_id)", NO_PARAMS)?;
        Ok(())
    }

    fn insert_dict(&self, db : &Connection, dict_id : &str, dict : Dictionary) -> Result<(),rusqlite::Error> {
        let mut stmt = db.prepare("INSERT INTO dictionaries (id, release, source_language, target_languages, genres, license, creators, publishers VALUES (?, ?, ?, ?, ?, ?, ?, ?)")?;

        stmt.execute(
            &[dict_id, &serde_json::to_string(&dict.release).unwrap(), 
              &dict.source_language,
              &serde_json::to_string(&dict.target_language).unwrap(),
              &serde_json::to_string(&dict.genre).unwrap(),
              &dict.license,
              &serde_json::to_string(&dict.creator).unwrap(),
              &serde_json::to_string(&dict.publisher).unwrap()])?;
        Ok(())
    }
    fn insert_entry(&self, db : &Connection, dict_id : &str, entry_content : EntryContent, release : Release) -> Result<(),rusqlite::Error> {
        let mut stmt = db.prepare("INSERT INTO entries (release, lemma, id, part_of_speech, format, dict) VALUES (?,?,?,?,?,?)")?;
        stmt.execute(&[
            &serde_json::to_string(&release).unwrap(),
            entry_content.lemma(),
            entry_content.id(),
            &serde_json::to_string(&entry_content.pos()).unwrap(),
            &serde_json::to_string(&entry_content.format()).unwrap()])?;

        let mut stmt2 = db.prepare("SELECT last_insert_rowid()")?;
        let mut result = stmt2.query(NO_PARAMS)?;

        if let Some(r) = result.next()? {
            let row_id : u32 = r.get(0)?;
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
                    let mut stmt4 = db.prepare("INSERT INTO json_entries (entry_id, json) VALUES(?,?)")?;
                    stmt4.execute(&[&format!("{}",row_id), &entry_content.content()])?;
                }
                EntryContent::OntoLex(_,_,_,_,_) => {
                    let mut stmt4 = db.prepare("INSERT INTO json_entries (entry_id, json) VALUES(?,?)")?;
                    stmt4.execute(&[&format!("{}",row_id), &entry_content.content()])?;
                }
            }
                    
            
            Ok(())
        } else {
            panic!("After INSERT did not return for last_insert_rowid()")    
        }

    }
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
        let db = Connection::open(&self.path)?;
        let mut stmt = db.prepare("SELECT release, source_language, target_languages, genres, license, creators, publishers FROM dictionaries WHERE id=?")?;
        let mut result = stmt.query(&[dictionary])?;
        
        if let Some(r) = result.next()? {
            let r_str : String = r.get(0)?;
            let release = Release::from_str(&r_str).map_err(|e| BackendError::Other(e))?;
            let source_lang = r.get(1)?;
            let tl_str : String = r.get(2)?;
            let targ_langs = tl_str.split(",").map(|x| x.to_string()).collect();
            let g_str : String = r.get(3)?;
            let genres = g_str.split(",").map(|x| Genre::from_str(x).expect("Bad genre string in DB")).collect();
            let license = r.get(4)?;
            let c_str : String = r.get(5)?;
            let creator_ids = c_str.split(",");
            let p_str : String = r.get(6)?;
            let publisher_ids = p_str.split(",");
            let mut creators = Vec::new();
            let mut publishers = Vec::new();

            let mut stmt2 = db.prepare("SELECT name, email, url FROM agents WHERE id=?")?;
            for creator_id in creator_ids {
                creators.push(stmt2.query_row(&[creator_id], |r| {
                    Ok(Agent { 
                        name: r.get(0)?,
                        email: empty_str_to_none(r.get(1)?),
                        url: empty_str_to_none(r.get(2)?)
                    })
                })?);
            }
            for publisher_id in publisher_ids {
                publishers.push(stmt2.query_row(&[publisher_id], |r| {
                    Ok(Agent { 
                        name: r.get(0)?,
                        email: empty_str_to_none(r.get(1)?),
                        url: empty_str_to_none(r.get(2)?)
                    })
                })?);
            }
            Ok(Dictionary::new(release, source_lang, targ_langs,
                    genres, license, creators, publishers))

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
                    db.prepare("SELECT release, lemma, id, part_of_speech, format FROM entries WHERE dict=? LIMIT ? OFFSET ?")?,
                None =>
                    db.prepare("SELECT release, lemma, id, part_of_speech, format FROM entries WHERE dict=? OFFSET ?")?
            },
            None => match limit {
                Some(_) =>
                    db.prepare("SELECT release, lemma, id, part_of_speech, format FROM entries WHERE dict=? LIMIT ?")?,
                None =>
                    db.prepare("SELECT release, lemma, id, part_of_speech, format FROM entries WHERE dict=?")?
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
            let format_str : String = r.get(4)?;
            entries.push(Entry {
                release: Release::from_str(&r_str).map_err(|e| BackendError::Other(e))?,
                lemma: r.get(1)?,
                id: r.get(2)?,
                part_of_speech: pos_str.split(",").flat_map(|x| PartOfSpeech::from_str(x)).collect(),
                formats: format_str.split(",").flat_map(|x| Format::from_str(x)).collect()
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
        let mut q = String::from("SELECT release, lemma, id, part_of_speech, format FROM entries");
        
        if inflected {
            q.push_str(" JOIN variants ON variant.lemma == entries.lemma WHERE dict=?");
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
            q.push_str(" AND variant.form=?");
        } else {
            q.push_str(" AND entries.lemma=?");
        }
        params.push(headword);
        let mut o_str = String::new();
        if let Some(o) = offset {
            q.push_str(" OFFSET ?");
            o_str.push_str(&format!("{}", o));
            params.push(&o_str);
        }
        let mut l_str = String::new();
        if let Some(l) = limit {
            q.push_str(" LIMIT ?");
            l_str.push_str(&format!("{}", l));
            params.push(&l_str);
        }

        let mut stmt = db.prepare(&q)?;
        let mut result = stmt.query(&params)?;
        let mut entries = Vec::new();
        while let Some(r) = result.next()? {
            let r_str : String = r.get(0)?;
            let pos_str : String = r.get(3)?;
            let format_str : String = r.get(4)?;
            entries.push(Entry {
                release: Release::from_str(&r_str).map_err(|e| BackendError::Other(e))?,
                lemma: r.get(1)?,
                id: r.get(2)?,
                part_of_speech: pos_str.split(",").flat_map(|x| PartOfSpeech::from_str(x)).collect(),
                formats: format_str.split(",").flat_map(|x| Format::from_str(x)).collect()
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
        let mut stmt = db.prepare("SELECT json FROM json_entries WHERE dict=? AND id=?")?;
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
        let mut stmt = db.prepare("SELECT ontolex FROM ontolex_entries WHERE dict=? AND id=?")?;
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
        let mut stmt = db.prepare("SELECT tei FROM tei_entries WHERE dict=? AND id=?")?;
        let mut result = stmt.query(&[dictionary, id])?;
        if let Some(r) = result.next()? {
            Ok(r.get(0)?)
        } else {
            Err(BackendError::NotFound)
        }
     }

}

fn empty_str_to_none(s : String) -> Option<String> {
    if s == "" {
        None
    } else {
        Some(s)
    }
}

#[test]
fn test_create_db() {
    let tmp_db_path = String::from("test-tmp.db");
    let state = RusqliteState::new(tmp_db_path.clone());
    state.load(Release::PUBLIC, HashMap::new(), HashMap::new()).unwrap();
    fs::remove_file("test-tmp.db").unwrap();
}

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::str::FromStr;

/// The backend access to a dictionary
pub trait Backend {
    /// List the identifiers for all dictionaries
    fn dictionaries(&self) -> Result<Vec<String>,BackendError>;
    /// Obtain the metadata about a given dictionary
    fn about(&self, dictionary : &str) -> Result<Dictionary,BackendError>;
    /// List all entries in a dictrionary
    fn list(&self, dictionary : &str, offset : Option<usize>, 
            limit : Option<usize>) -> Result<Vec<Entry>,BackendError>;
    /// Search the dictionary by headword
    fn lookup(&self, dictionary : &str, headword : &str,
              offset : Option<usize>, limit : Option<usize>,
              part_of_speech : Option<PartOfSpeech>, inflected : bool) -> Result<Vec<Entry>,BackendError>;
    /// Get the content as Json
    fn entry_json(&self, dictionary : &str, id : &str) -> Result<JsonEntry,BackendError>;
    /// Get the content as OntoLex
    fn entry_ontolex(&self, dictionary : &str, id : &str) -> Result<String,BackendError>;
    /// Get the content as TEI
    fn entry_tei(&self, dictionary : &str, id : &str) -> Result<String,BackendError>;
}

quick_error! {
    #[derive(Debug)]
    pub enum BackendError {
        NotFound {}
        Sqlite(err : rusqlite::Error) {
            from()
        }
        Json(err : serde_json::Error) {
            from()
        }
        Other(err : String) {
            description(err)
        }
    }
}


#[derive(Clone,StateData)]
pub struct EDSState {
    dictionaries : Arc<Mutex<HashMap<String,Dictionary>>>,
    entries_lemmas : Arc<Mutex<HashMap<String,HashMap<String,Vec<Entry>>>>>,
    entries_forms : Arc<Mutex<HashMap<String,HashMap<String,Vec<Entry>>>>>,
    entries_id : Arc<Mutex<HashMap<String,HashMap<String,EntryContent>>>>
}

impl EDSState {
    pub fn new(release : Release,
               dictionaries : HashMap<String, Dictionary>,
               dict_entries : HashMap<String, Vec<EntryContent>>) -> Self {
        let mut dict_entry_map = HashMap::new();
        let mut dict_entry_map2 = HashMap::new();
        let mut entry_by_id = HashMap::new();
        for (id, entries) in dict_entries {
            let mut entry_map = HashMap::new();
            let mut eid_map = HashMap::new();
            let mut entry_map2 = HashMap::new();
            for entry in entries {
                eid_map.insert(entry.id().to_string(), entry.clone());
                if !entry_map.contains_key(entry.lemma()) {
                    entry_map.insert(entry.lemma().to_string(),
                        Vec::new());
                }
                entry_map.entry(entry.lemma().to_string())
                    .and_modify(|e| e.push(entry_from_content(release.clone(), &entry)));
                for var in entry.variants() {
                    if !entry_map2.contains_key(&var) {
                        entry_map2.insert(var.to_string(),
                        Vec::new());
                    }
                    entry_map.entry(var.clone())
                        .and_modify(|e| e.push(entry_from_content(release.clone(), &entry)));
                    }
            }
            dict_entry_map.insert(id.clone(), entry_map);
            dict_entry_map2.insert(id.clone(), entry_map2);
            entry_by_id.insert(id, eid_map);
        }
        EDSState {
            dictionaries : Arc::new(Mutex::new(dictionaries)),
            entries_lemmas : Arc::new(Mutex::new(dict_entry_map)),
            entries_forms : Arc::new(Mutex::new(dict_entry_map2)),
            entries_id : Arc::new(Mutex::new(entry_by_id))
        }
    }
}

impl Backend for EDSState {
    fn dictionaries(&self) -> Result<Vec<String>,BackendError> {
        Ok(self.dictionaries.lock().unwrap().keys().map(|x| x.to_string()).collect())
    }
    fn about(&self, dictionary : &str) -> Result<Dictionary,BackendError> {
        self.dictionaries.lock().unwrap().get(dictionary).map(|x| x.clone())
            .ok_or(BackendError::NotFound)
    }   
    fn list(&self, dictionary : &str, offset : Option<usize>,
            limit : Option<usize>) -> Result<Vec<Entry>,BackendError> {
        match self.entries_lemmas.lock().unwrap().get(dictionary) {
            Some(emap) => {
                let entries : Vec<Entry> = match offset {
                    Some(offset) => {
                        match limit {
                            Some(limit) => 
                                emap.values().flat_map(|x| x).skip(offset).take(limit).map(|x| x.clone()).collect(),
                            None =>
                                emap.values().flat_map(|x| x).skip(offset).map(|x| x.clone()).collect()
                        }
                    },
                    None =>
                        match limit {
                            Some(limit) => 
                                emap.values().flat_map(|x| x).take(limit).map(|x| x.clone()).collect(),
                            None =>
                                emap.values().flat_map(|x| x).map(|x| x.clone()).collect()
                        }
                };
                Some(entries).ok_or(BackendError::NotFound)
            },
            None => {
                None.ok_or(BackendError::NotFound)
            }
        }
    }
    fn lookup(&self, dictionary : &str, headword : &str,
              offset : Option<usize>, limit : Option<usize>,
              part_of_speech : Option<PartOfSpeech>, inflected : bool) -> Result<Vec<Entry>,BackendError> {
        let dict = self.entries_lemmas.lock().unwrap();
        let dict2 = self.entries_forms.lock().unwrap();
        match dict.get(dictionary).and_then(|x| x.get(headword)) {
            Some(emap) => {
                let i1 = emap.iter()
                    .filter(|e| part_of_speech.is_none() || e.part_of_speech.contains(part_of_speech.as_ref().unwrap()));
                let el = Vec::new();
                let i2 = (if inflected {
                    match dict2.get(dictionary).and_then(|x| x.get(headword)) {
                        Some(emap2) => {
                            emap2.iter()
                        },
                        None => {
                            el.iter()
                        }
                    }
                } else {
                    el.iter()
                }).filter(|e| part_of_speech.is_none() || e.part_of_speech.contains(part_of_speech.as_ref().unwrap()));
                let entries : Vec<Entry> = match offset {
                    Some(offset) => {
                        match limit {
                            Some(limit) => 
                                i1.chain(i2).skip(offset).take(limit).map(|x| x.clone()).collect(),
                            None =>
                                i1.chain(i2).skip(offset).map(|x| x.clone()).collect()
                        }
                    },
                    None =>
                        match limit {
                            Some(limit) => 
                                i1.chain(i2).take(limit).map(|x| x.clone()).collect(),
                            None =>
                                i1.chain(i2).map(|x| x.clone()).collect()
                        }
                };
                Some(entries)
            .ok_or(BackendError::NotFound)
            }
            None => {
                None
            .ok_or(BackendError::NotFound)
            }
        }
    }
    fn entry_json(&self, dictionary : &str, id : &str) -> Result<JsonEntry,BackendError> {
        self.entries_id.lock().unwrap().get(dictionary).and_then(|x| match x.get(id) {
            Some(EntryContent::Json(entry)) => Some(entry.clone()),
            _ => None
        })
            .ok_or(BackendError::NotFound)
    }
    fn entry_ontolex(&self, _dictionary : &str, _id : &str) -> Result<String,BackendError> { panic!("TODO") }
    fn entry_tei(&self, dictionary : &str, id : &str) -> Result<String,BackendError> { 
        self.entries_id.lock().unwrap().get(dictionary).and_then(|x| match x.get(id) {
            Some(EntryContent::Tei(_,_,_,_,content)) => Some(content.clone()),
            _ => None
        }).ok_or(BackendError::NotFound)
    }

}

pub fn entry_from_content(release : Release, content : &EntryContent) -> Entry {
    Entry {
        release: release,
        lemma: content.lemma().to_string(),
        id: content.id().to_string(),
        part_of_speech: content.pos(),
        formats: vec![content.format()]
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dictionary {
    pub release : Release,
    pub source_language : String,
    pub target_language : Vec<String>,
    pub genre : Vec<Genre>,
    pub license : String,
    pub creator : Vec<Agent>,
    pub publisher : Vec<Agent>
}

impl Dictionary {
    pub fn new(release : Release, source_language : String,
               target_language : Vec<String>,
               genre : Vec<Genre>,
               license : String,
               creator : Vec<Agent>,
               publisher : Vec<Agent>) -> Self {
        Dictionary {
            release, source_language, target_language, genre,
            license, creator, publisher
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[allow(non_camel_case_types)]
pub enum Release {
    PUBLIC,
    NONCOMMERCIAL,
    RESEARCH,
    PRIVATE
}

impl FromStr for Release {
    type Err = String;

    fn from_str(s: &str) -> Result<Release, String> {
        match s {
            "PUBLIC" => Ok(Release::PUBLIC),
            "NONCOMMERCIAL" => Ok(Release::NONCOMMERCIAL),
            "RESEARCH" => Ok(Release::RESEARCH),
            "PRIVATE" => Ok(Release::PRIVATE),
            _ => Err(format!("Bad value for release: {}", s))
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[allow(non_camel_case_types)]
pub enum Genre {
    gen,
    lrn,
    ety,
    spe,
    his,
    ort,
    trm
}

impl FromStr for Genre {
    type Err = String;

    fn from_str(s: &str) -> Result<Genre, String> {
        match s {
            "gen" => Ok(Genre::gen),
            "lrn" => Ok(Genre::lrn),
            "ety" => Ok(Genre::ety),
            "spe" => Ok(Genre::spe),
            "his" => Ok(Genre::his),
            "ort" => Ok(Genre::ort),
            "trm" => Ok(Genre::trm),
            _ => Err(format!("Not a valid genre: {}", s))
        }
    }
} 


#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Agent {
    pub name : String,
    pub email : Option<String>,
    pub url : Option<String>
}

impl Agent {
    pub fn new() -> Self { 
        Self {
            name: String::new(),
            email: None,
            url: None
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct DCProperty {
    name : String,
    value : String
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub release : Release,
    pub lemma : String,
    pub id : String,
    pub part_of_speech : Vec<PartOfSpeech>,
    pub formats : Vec<Format>
}

impl Entry {
    pub fn new(release : Release, lemma : String, id : String,
               part_of_speech : Vec<PartOfSpeech>, formats : Vec<Format>) -> Self {
        Entry {
            release, lemma, id, part_of_speech, formats
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
#[allow(non_camel_case_types)]
pub enum PartOfSpeech {
    ADJ,
    ADP,
    ADV,
    AUX,
    CCONJ,
    DET,
    INTJ,
    NOUN,
    NUM,
    PART,
    PRON,
    PROPN,
    PUNCT,
    SCONJ,
    SYM,
    VERB,
    X
}

impl FromStr for PartOfSpeech {
    type Err = String;

    fn from_str(s: &str) -> Result<PartOfSpeech, String> {
        match s {
            "ADJ" => Ok(PartOfSpeech::ADJ),
            "ADP" => Ok(PartOfSpeech::ADP),
            "ADV" => Ok(PartOfSpeech::ADV),
            "AUX" => Ok(PartOfSpeech::AUX),
            "CCONJ" => Ok(PartOfSpeech::CCONJ),
            "DET" => Ok(PartOfSpeech::DET),
            "INTJ" => Ok(PartOfSpeech::INTJ),
            "NOUN" => Ok(PartOfSpeech::NOUN),
            "NUM" => Ok(PartOfSpeech::NUM),
            "PART" => Ok(PartOfSpeech::PART),
            "PRON" => Ok(PartOfSpeech::PRON),
            "PROPN" => Ok(PartOfSpeech::PROPN),
            "PUNCT" => Ok(PartOfSpeech::PUNCT),
            "SCONJ" => Ok(PartOfSpeech::SCONJ),
            "SYM" => Ok(PartOfSpeech::SYM),
            "VERB" => Ok(PartOfSpeech::VERB),
            "X" => Ok(PartOfSpeech::X),
            _ => Err(format!("Not a valid part of speech: {}", s))
        }
    }
}
 

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
#[allow(non_camel_case_types)]
pub enum Format {
    tei,
    ontolex,
    json
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Format, String> {
        match s {
            "tei" => Ok(Format::tei),
            "ontolex" => Ok(Format::ontolex),
            "json" => Ok(Format::json),
            _ => Err(format!("Bad format: {}", s))
        } 
    }
}


#[derive(Clone,Debug,Deserialize)]
pub enum EntryContent {
    Json(JsonEntry),
    Tei(String, String, Vec<PartOfSpeech>, Vec<String>, String),
    OntoLex(String, String, Vec<PartOfSpeech>, Vec<String>, String)
}

impl EntryContent {
    pub fn id(&self) -> &str {
        match self {
            EntryContent::Json(j) => &j.id,
            EntryContent::Tei(id,_,_,_,_) => id,
            EntryContent::OntoLex(id,_,_,_,_) => id
        }
    }

    pub fn lemma(&self) -> &str {
        match self {
            EntryContent::Json(j) => &j.canonical_form.written_rep,
            EntryContent::Tei(_,lemma,_,_,_) => lemma,
            EntryContent::OntoLex(_,lemma,_,_,_) => lemma
        }
    }

    pub fn pos(&self) -> Vec<PartOfSpeech> {
        match self { 
            EntryContent::Json(j) => vec![JsonPartOfSpeech::convert(&j.part_of_speech)],
            EntryContent::Tei(_,_,pos,_,_) => pos.clone(),
            EntryContent::OntoLex(_,_,pos,_,_) => pos.clone()
        }
    }
    pub fn format(&self) -> Format {
        match self {
            EntryContent::Json(_) => Format::json,
            EntryContent::Tei(_,_,_,_,_) => Format::tei,
            EntryContent::OntoLex(_,_,_,_,_) => Format::ontolex
        }
    }
    pub fn variants(&self) -> Vec<String> {
        match self {
            EntryContent::Json(j) => if let Some(ref forms) = j.other_form {
                forms.iter().map(|x| x.written_rep.to_string()).collect()
            } else {
                Vec::new()
            },
            EntryContent::Tei(_,_,_,vars,_) => vars.clone(),
            EntryContent::OntoLex(_,_,_,vars,_) => vars.clone()
        }
    }

    pub fn content(&self) -> String {
        match self {
            EntryContent::Json(j) => serde_json::to_string(j).unwrap(),
            EntryContent::Tei(_,_,_,_,content) => content.clone(),
            EntryContent::OntoLex(_,_,_,_,content) => content.clone()
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonEntry {
    #[serde(rename="@context")] context : String,
    #[serde(rename="@id")] pub id : String,
    #[serde(rename="@type")] pub entry_type : Type,
    pub canonical_form : Form,
    pub part_of_speech : JsonPartOfSpeech,
    pub other_form : Option<Vec<Form>>,
    pub morphological_pattern : Option<String>,
    pub etymology : Option<String>,
    pub senses : Vec<Sense>,
    pub usage : Option<String>
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub enum Type {
    LexicalEntry,
    Word,
    MultiWordExpression,
    Affix
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Form {
    written_rep : String,
    phonetic_rep : Option<String>
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub enum JsonPartOfSpeech {
    #[serde(rename="adjective")] Adjective,
    #[serde(rename="adposition")] Adposition,
    #[serde(rename="adverb")] Adverb,
    #[serde(rename="auxiliary")] Auxiliary,
    #[serde(rename="coordinatingConjunction")] CoordinatingConjunction,
    #[serde(rename="determiner")] Determiner,
    #[serde(rename="interjection")] Interjection,
    #[serde(rename="commonNoun")] CommonNoun,
    #[serde(rename="numeral")] Numeral,
    #[serde(rename="particle")] Particle,
    #[serde(rename="properNoun")] ProperNoun,
    #[serde(rename="punctuation")] Punctuation,
    #[serde(rename="subordinatingConjunction")] SubordinatingConjunction,
    #[serde(rename="symbol")] Symbol,
    #[serde(rename="verb")] Verb,
    #[serde(rename="other")] Other
}

impl JsonPartOfSpeech {
    pub fn convert(&self) -> PartOfSpeech {
        match self {
            JsonPartOfSpeech::Adjective => PartOfSpeech::ADJ,
            JsonPartOfSpeech::Adposition => PartOfSpeech::ADP,
            JsonPartOfSpeech::Adverb => PartOfSpeech::ADV,
            JsonPartOfSpeech::Auxiliary => PartOfSpeech::AUX,
            JsonPartOfSpeech::CoordinatingConjunction => PartOfSpeech::CCONJ,
            JsonPartOfSpeech::Determiner => PartOfSpeech::DET,
            JsonPartOfSpeech::Interjection => PartOfSpeech::INTJ,
            JsonPartOfSpeech::CommonNoun => PartOfSpeech::NOUN,
            JsonPartOfSpeech::Numeral => PartOfSpeech::NUM,
            JsonPartOfSpeech::Particle => PartOfSpeech::PART,
            JsonPartOfSpeech::ProperNoun => PartOfSpeech::PROPN,
            JsonPartOfSpeech::Punctuation => PartOfSpeech::PUNCT,
            JsonPartOfSpeech::SubordinatingConjunction => PartOfSpeech::SCONJ,
            JsonPartOfSpeech::Symbol => PartOfSpeech::SYM,
            JsonPartOfSpeech::Verb => PartOfSpeech::VERB,
            JsonPartOfSpeech::Other => PartOfSpeech::X
        }
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Sense {
    definition : Option<String>,
    reference : Option<String>
}


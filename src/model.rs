use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Clone,StateData)]
pub struct EDSState {
    pub dictionaries : Arc<Mutex<HashMap<String,Dictionary>>>,
    pub entries_lemmas : Arc<Mutex<HashMap<String,HashMap<String,Vec<Entry>>>>>,
    pub entries_forms : Arc<Mutex<HashMap<String,HashMap<String,Vec<Entry>>>>>,
    pub entries_id : Arc<Mutex<HashMap<String,HashMap<String,EntryContent>>>>
}

impl EDSState {
    pub fn new(dictionaries : HashMap<String, Dictionary>,
               dict_entries : HashMap<String, Vec<JsonEntry>>) -> Self {
        let mut dict_entry_map = HashMap::new();
        let mut dict_entry_map2 = HashMap::new();
        let mut entry_by_id = HashMap::new();
        for (id, entries) in dict_entries {
            let mut entry_map = HashMap::new();
            let mut eid_map = HashMap::new();
            let mut entry_map2 = HashMap::new();
            for entry in entries {
                eid_map.insert(entry.id.clone(), EntryContent::Json(entry.clone()));
                if !entry_map.contains_key(&entry.canonical_form.written_rep) {
                    entry_map.insert(entry.canonical_form.written_rep.to_string(),
                        Vec::new());
                }
                entry_map.entry(entry.canonical_form.written_rep.clone())
                    .and_modify(|e| e.push(entry_from_content(&entry)));
                match entry.other_form.as_ref() {
                    Some(of) => {
                        for form in of {
                            if !entry_map2.contains_key(&form.written_rep) {
                                entry_map2.insert(form.written_rep.to_string(),
                                    Vec::new());
                            }
                            entry_map.entry(form.written_rep.clone())
                                .and_modify(|e| e.push(entry_from_content(&entry)));
            
                        }
                    },
                    None => {}
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

fn entry_from_content(content : &JsonEntry) -> Entry {
    Entry {
        release: Release::PUBLIC,
        lemma: content.canonical_form.written_rep.to_string(),
        id: content.id.to_string(),
        part_of_speech: vec![content.part_of_speech.convert()],
        formats: vec![Format::json]
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dictionary {
    release : Release,
    source_language : String,
    target_language : Vec<String>,
    genre : Vec<Genre>,
    license : String,
    creator : Vec<Agent>,
    publisher : Vec<Agent>
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
    release : Release,
    lemma : String,
    id : String,
    pub part_of_speech : Vec<PartOfSpeech>,
    formats : Vec<Format>
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

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
#[allow(non_camel_case_types)]
pub enum Format {
    tei,
    ontolex,
    json
}

#[derive(Clone,Debug,Deserialize)]
pub enum EntryContent {
    Json(JsonEntry),
    Tei(String),
    OntoLex(String)
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

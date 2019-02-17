use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Clone,StateData)]
pub struct EDSState {
    pub dictionaries : Arc<Mutex<HashMap<String,Dictionary>>>,
    pub entries : Arc<Mutex<HashMap<String,HashMap<String,Vec<EntryContent>>>>>,
    pub entries_by_id : Arc<Mutex<HashMap<String,HashMap<String,EntryContent>>>>
}

impl EDSState {
    pub fn new(dictionaries : HashMap<String, Dictionary>,
               dict_entries : HashMap<String, Vec<EntryContent>>) -> Self {
        let mut dict_entry_map = HashMap::new();
        let mut entry_by_id = HashMap::new();
        for (id, entries) in dict_entries {
            let mut entry_map = HashMap::new();
            let mut eid_map = HashMap::new();
            for entry in entries {
                if !entry_map.contains_key(&entry.canonical_form.written_rep) {
                    entry_map.insert(entry.canonical_form.written_rep.to_string(),
                        Vec::new());
                }
                eid_map.insert(entry.id.clone(), entry.clone());
                entry_map.entry(entry.canonical_form.written_rep.clone())
                    .and_modify(|e| e.push(entry));
            }
            dict_entry_map.insert(id.clone(), entry_map);
            entry_by_id.insert(id, eid_map);
        }
        EDSState {
            dictionaries : Arc::new(Mutex::new(dictionaries)),
            entries : Arc::new(Mutex::new(dict_entry_map)),
            entries_by_id : Arc::new(Mutex::new(entry_by_id))
        }
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
    publisher : Vec<Agent>,
    //properties : Vec<DCProperty>,
    entries : Vec<Entry>
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
    name : String,
    email : Option<String>,
    url : Option<String>
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
    language : String,
    id : String,
    part_of_speech : Vec<PartOfSpeech>,
    formats : Vec<String>
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

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryContent {
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
    #[serde(rename="lexinfo:adjective")] Adjective,
    #[serde(rename="lexinfo:adposition")] Adposition,
    #[serde(rename="lexinfo:adverb")] Adverb,
    #[serde(rename="lexinfo:auxiliary")] Auxiliary,
    #[serde(rename="lexinfo:coordinatingConjunction")] CoordinatingConjunction,
    #[serde(rename="lexinfo:determiner")] Determiner,
    #[serde(rename="lexinfo:interjection")] Interjection,
    #[serde(rename="lexinfo:commonNoun")] CommonNoun,
    #[serde(rename="lexinfo:numeral")] Numeral,
    #[serde(rename="lexinfo:particle")] Particle,
    #[serde(rename="lexinfo:properNoun")] ProperNoun,
    #[serde(rename="lexinfo:punctuation")] Punctuation,
    #[serde(rename="lexinfo:subordinatingConjunction")] SubordinatingConjunction,
    #[serde(rename="lexinfo:symbol")] Symbol,
    #[serde(rename="lexinfo:verb")] Verb,
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


use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::str::FromStr;

type Date = String;
type DateTime = String;

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
        Rdf(err : crate::rdf::turtle::TurtleParserError) {
            from()
        }
        Io(err : std::io::Error) {
            from()
        }
        OntoLex(msg : String) {
            description(msg)
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
    fn entry_ontolex(&self, dictionary : &str, id : &str) -> Result<String,BackendError> { 
        self.entries_id.lock().unwrap().get(dictionary).and_then(|x| match x.get(id) {
            Some(EntryContent::OntoLex(_,_,_,_,content)) => Some(content.clone()),
            _ => None
        }).ok_or(BackendError::NotFound)
    }
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
    #[serde(default)]
    pub creator : Vec<Agent>,
    #[serde(default)]
    pub publisher : Vec<Agent>,
    /// A summary of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "abstract")]
    pub _abstract : Option<String>,
    /// The method by which items are added to a collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accrual_method : Option<String>,

    /// The frequency with which items are added to a collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accrual_periodicity   : Option<String>,

    /// The policy governing the addition of items to a collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accrual_policy : Option<String>,

    /// An alternative name for the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative  : Option<String>,

    /// A class of entity for whom the resource is intended or useful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience     : Option<String>,

    /// Date that the resource became or will become available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available    : Option<Date>,

    /// A bibliographic reference for the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bibliographic_citation : Option<String>,

    /// An established standard to which the described resource conforms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conforms_to   : Option<String>,

    /// An entity responsible for making contributions to the resource.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub contributor  : Vec<Agent>,

    /// The spatial or temporal topic of the resource, the spatial applicability of the resource, or the jurisdiction under which the resource is relevant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage     : Option<String>,

    /// Date of creation of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created : Option<Date>,

    /// A point or period of time associated with an event in the lifecycle of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date : Option<DateTime>,

    /// Date of acceptance of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_accepted : Option<Date>,

    /// Date of copyright.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_copyrighted : Option<Date>,

    /// Date of submission of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_submitted : Option<Date>,

    /// An account of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description  : Option<String>,

    /// A class of entity, defined in terms of progression through an educational or training context, for which the described resource is intended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub education_level : Option<String>,

    /// The size or duration of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extent : Option<String>,

    /// A related resource that is substantially the same as the pre-existing described resource, but in another format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_format    : Option<String>,

    /// A related resource that is included either physically or logically in the described resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_part : Option<String>,

    /// A related resource that is a version, edition, or adaptation of the described resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_version   : Option<String>,

    /// An unambiguous reference to the resource within a given context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier   : Option<String>,

    /// A process, used to engender knowledge, attitudes and skills, that the described resource is designed to support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructional_method  : Option<String>,

    /// A related resource that is substantially the same as the described resource, but in another format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_format_of   : Option<String>,

    /// A related resource in which the described resource is physically or logically included.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_part_of     : Option<String>,

    /// A related resource that references, cites, or otherwise points to the described resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_referenced_by : Option<String>,

    /// A related resource that supplants, displaces, or supersedes the described resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_replaced_by : Option<String>,

    /// A related resource that requires the described resource to support its function, delivery, or coherence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_required_by : Option<String>,

    /// Date of formal issuance (e.g., publication) of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued : Option<Date>,

    /// A related resource of which the described resource is a version, edition, or adaptation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_version_of  : Option<String>,

    /// An entity that mediates access to the resource and for whom the resource is intended or useful.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub mediator     : Vec<Agent>,

    /// Date on which the resource was changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified     : Option<Date>,

    /// A statement of any changes in ownership and custody of the resource since its creation that are significant for its authenticity, integrity, and interpretation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance   : Option<String>,

    /// A related resource that is referenced, cited, or otherwise pointed to by the described resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references   : Option<String>,

    /// A related resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation     : Option<String>,

    /// A related resource that is supplanted, displaced, or superseded by the described resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaces     : Option<String>,

    /// A related resource that is required by the described resource to support its function, delivery, or coherence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires     : Option<String>,

    /// Information about rights held in and over the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rights : Option<String>,

    /// A person or organization owning or managing rights over the resource.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub rights_holder : Vec<Agent>,

    /// A related resource from which the described resource is derived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source : Option<String>,

    /// Spatial characteristics of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spatial : Option<String>,

    /// The topic of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject : Option<String>,

    /// A list of subunits of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_of_contents : Option<String>,

    /// Temporal characteristics of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporal     : Option<String>,

    /// The nature or genre of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub _type : Option<String>,

    /// Date of validity of a resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid : Option<Date>
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
            license, creator, publisher, 
            _abstract : None,
            accrual_method : None,
            accrual_periodicity   : None,
            accrual_policy : None,
            alternative  : None,
            audience     : None,
            available    : None,
            bibliographic_citation : None,
            conforms_to   : None,
            contributor  : Vec::new(),
            coverage     : None,
            created : None,
            date : None,
            date_accepted : None,
            date_copyrighted : None,
            date_submitted : None,
            description  : None,
            education_level : None,
            extent : None,
            has_format    : None,
            has_part : None,
            has_version   : None,
            identifier   : None,
            instructional_method  : None,
            is_format_of   : None,
            is_part_of     : None,
            is_referenced_by : None,
            is_replaced_by : None,
            is_required_by : None,
            issued : None,
            is_version_of  : None,
            mediator     : Vec::new(),
            modified     : None,
            provenance   : None,
            references   : None,
            relation     : None,
            replaces     : None,
            requires     : None,
            rights : None,
            rights_holder : Vec::new(),
            source : None,
            spatial : None,
            subject : None,
            table_of_contents : None,
            temporal     : None,
            _type : None,
            valid : None,
        }
    }

    pub fn set_dc_prop(&mut self, prop : &str, value : &str) {
        match prop {
            "abstract" => self._abstract = Some(value.to_owned()),
            "accrualMethod" => self.accrual_method = Some(value.to_owned()),
            "accrualPeriodicity" => self.  accrual_periodicity = Some(value.to_owned()),
            "accrualPolicy" => self.accrual_policy = Some(value.to_owned()),
            "alternative" => self. alternative = Some(value.to_owned()),
            "audience" => self.    audience = Some(value.to_owned()),
            "available" => self.   available = Some(value.to_owned()),
            "bibliographicCitation" => self.bibliographic_citation = Some(value.to_owned()),
            "conformsTo" => self.  conforms_to = Some(value.to_owned()),
            "contributor" => self. contributor = serde_json::from_str(value).unwrap_or_else(|_| Vec::new()),
            "coverage" => self.    coverage = Some(value.to_owned()),
            "created" => self.created = Some(value.to_owned()),
            "date" => self.date = Some(value.to_owned()),
            "dateAccepted" => self.date_accepted = Some(value.to_owned()),
            "dateCopyrighted" => self.date_copyrighted = Some(value.to_owned()),
            "dateSubmitted" => self.date_submitted = Some(value.to_owned()),
            "description" => self. description = Some(value.to_owned()),
            "educationLevel" => self.education_level = Some(value.to_owned()),
            "extent" => self.extent = Some(value.to_owned()),
            "hasFormat" => self.   has_format = Some(value.to_owned()),
            "hasPart" => self.has_part = Some(value.to_owned()),
            "hasVersion" => self.  has_version = Some(value.to_owned()),
            "identifier" => self.  identifier = Some(value.to_owned()),
            "instructionalMethod" => self. instructional_method = Some(value.to_owned()),
            "isFormatOf" => self.  is_format_of = Some(value.to_owned()),
            "isPartOf" => self.    is_part_of = Some(value.to_owned()),
            "isReferencedBy" => self.is_referenced_by = Some(value.to_owned()),
            "isReplacedBy" => self.is_replaced_by = Some(value.to_owned()),
            "isRequiredBy" => self.is_required_by = Some(value.to_owned()),
            "issued" => self.issued = Some(value.to_owned()),
            "isVersionOf" => self. is_version_of = Some(value.to_owned()),
            "mediator" => self.    mediator = serde_json::from_str(value).unwrap_or_else(|_| Vec::new()),
            "modified" => self.    modified = Some(value.to_owned()),
            "provenance" => self.  provenance = Some(value.to_owned()),
            "references" => self.  references = Some(value.to_owned()),
            "relation" => self.    relation = Some(value.to_owned()),
            "replaces" => self.    replaces = Some(value.to_owned()),
            "requires" => self.    requires = Some(value.to_owned()),
            "rights" => self.rights = Some(value.to_owned()),
            "rightsHolder" => self.rights_holder = serde_json::from_str(value).unwrap_or_else(|_| Vec::new()),
            "source" => self.source = Some(value.to_owned()),
            "spatial" => self.spatial = Some(value.to_owned()),
            "subject" => self.subject = Some(value.to_owned()),
            "tableOfContents" => self.table_of_contents = Some(value.to_owned()),
            "temporal" => self.    temporal = Some(value.to_owned()),
            "type" => self._type = Some(value.to_owned()),
            "valid" => self.valid = Some(value.to_owned()),
            _ => eprintln!("Unrecognised DC property ({}) ignored", prop)
        }
    }

    pub fn get_dc_props(&self) -> Vec<(&'static str, String)> {
        let mut props = Vec::new();
        if let Some(ref value) = self._abstract {
            props.push(("abstract",value.to_owned()));
        }
        if let Some(ref value) = self.accrual_method {
            props.push(("accrualMethod",value.to_owned()));
        }
        if let Some(ref value) = self.  accrual_periodicity {
            props.push(("accrualPeriodicity",value.to_owned()));
        }
        if let Some(ref value) = self.accrual_policy {
            props.push(("accrualPolicy",value.to_owned()));
        }
        if let Some(ref value) = self. alternative {
            props.push(("alternative",value.to_owned()));
        }
        if let Some(ref value) = self.    audience {
            props.push(("audience",value.to_owned()));
        }
        if let Some(ref value) = self.   available {
            props.push(("available",value.to_owned()));
        }
        if let Some(ref value) = self.bibliographic_citation {
            props.push(("bibliographicCitation",value.to_owned()));
        }
        if let Some(ref value) = self.  conforms_to {
            props.push(("conformsTo",value.to_owned()));
        }
        for value in self.contributor.iter() {
            props.push(("contributor",serde_json::to_string(value).unwrap_or("".to_owned())));
        }
        if let Some(ref value) = self.    coverage {
            props.push(("coverage",value.to_owned()));
        }
        if let Some(ref value) = self.created {
            props.push(("created",value.to_owned()));
        }
        if let Some(ref value) = self.date {
            props.push(("date",value.to_owned()));
        }
        if let Some(ref value) = self.date_accepted {
            props.push(("dateAccepted",value.to_owned()));
        }
        if let Some(ref value) = self.date_copyrighted {
            props.push(("dateCopyrighted",value.to_owned()));
        }
        if let Some(ref value) = self.date_submitted {
            props.push(("dateSubmitted",value.to_owned()));
        }
        if let Some(ref value) = self. description {
            props.push(("description",value.to_owned()));
        }
        if let Some(ref value) = self.education_level {
            props.push(("educationLevel",value.to_owned()));
        }
        if let Some(ref value) = self.extent {
            props.push(("extent",value.to_owned()));
        }
        if let Some(ref value) = self.   has_format {
            props.push(("hasFormat",value.to_owned()));
        }
        if let Some(ref value) = self.has_part {
            props.push(("hasPart",value.to_owned()));
        }
        if let Some(ref value) = self.  has_version {
            props.push(("hasVersion",value.to_owned()));
        }
        if let Some(ref value) = self.  identifier {
            props.push(("identifier",value.to_owned()));
        }
        if let Some(ref value) = self. instructional_method {
            props.push(("instructionalMethod",value.to_owned()));
        }
        if let Some(ref value) = self.  is_format_of {
            props.push(("isFormatOf",value.to_owned()));
        }
        if let Some(ref value) = self.    is_part_of {
            props.push(("isPartOf",value.to_owned()));
        }
        if let Some(ref value) = self.is_referenced_by {
            props.push(("isReferencedBy",value.to_owned()));
        }
        if let Some(ref value) = self.is_replaced_by {
            props.push(("isReplacesBy",value.to_owned()));
        }
        if let Some(ref value) = self.is_required_by {
            props.push(("isRequiredBy",value.to_owned()));
        }
        if let Some(ref value) = self.issued {
            props.push(("issued",value.to_owned()));
        }
        if let Some(ref value) = self. is_version_of {
            props.push(("isVersionOf",value.to_owned()));
        }
        for value in self.mediator.iter() {
            props.push(("mediator",serde_json::to_string(value).unwrap_or("".to_owned())));
        }
        if let Some(ref value) = self.    modified {
            props.push(("modified",value.to_owned()));
        }
        if let Some(ref value) = self.  provenance {
            props.push(("provenance",value.to_owned()));
        }
        if let Some(ref value) = self.  references {
            props.push(("references",value.to_owned()));
        }
        if let Some(ref value) = self.    relation {
            props.push(("relation",value.to_owned()));
        }
        if let Some(ref value) = self.    replaces {
            props.push(("replaces",value.to_owned()));
        }
        if let Some(ref value) = self.    requires {
            props.push(("requires",value.to_owned()));
        }
        if let Some(ref value) = self.rights {
            props.push(("rights",value.to_owned()));
        }
        for value in self.rights_holder.iter() {
            props.push(("rightsHolder",serde_json::to_string(value).unwrap_or("".to_owned())));
        }
        if let Some(ref value) = self.source {
            props.push(("source",value.to_owned()));
        }
        if let Some(ref value) = self.spatial {
            props.push(("spatial",value.to_owned()));
        }
        if let Some(ref value) = self.subject {
            props.push(("subject",value.to_owned()));
        }
        if let Some(ref value) = self.table_of_contents {
            props.push(("tableOfContents",value.to_owned()));
        }
        if let Some(ref value) = self.    temporal {
            props.push(("temporal",value.to_owned()));
        }
        if let Some(ref value) = self._type {
            props.push(("type",value.to_owned()));
        }
        if let Some(ref value) = self.valid {
            props.push(("valid",value.to_owned()));
        }

        props
    }
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
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

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
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


#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Agent {
    pub name : String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email : Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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


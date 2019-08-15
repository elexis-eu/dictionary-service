use std::io::Read;
use crate::model::{Release, Genre, Dictionary, EntryContent, PartOfSpeech,BackendError,Agent};
use crate::BackendImpl;
use std::collections::HashMap;
use std::iter::Peekable;
use crate::rdf::turtle::parse_turtle;
use crate::rdf::model::{NamedNode,Value,Resource,Triple,Namespace,Literal};

fn make_id(s : &str) -> String {
    let e1 : Vec<&str> = s.split("#").collect();
    if e1.len() > 1 {
        e1[e1.len() - 1].to_owned()
    } else {
        let e2 : Vec<&str> = s.split("/").collect();
        if e2.len() > 1 {
            e2[e2.len() - 1].to_owned()
        } else {
            s.to_owned()
        }
    }
}

pub fn parse<R : Read, F>(mut input : R, release : Release,
    genre : Vec<Genre>, foo : F) -> Result<BackendImpl,BackendError>
    where F : FnOnce(Release, HashMap<String, Dictionary>, HashMap<String, Vec<EntryContent>>) -> Result<BackendImpl,BackendError> {
        let mut content = String::new();
        input.read_to_string(&mut content)?;
        parse_str(&content, release, genre, foo)
}

pub fn parse_str<F>(content : &str, release : Release,
    genre : Vec<Genre>, foo : F) -> Result<BackendImpl,BackendError>
    where F : FnOnce(Release, HashMap<String, Dictionary>, HashMap<String, Vec<EntryContent>>) -> Result<BackendImpl,BackendError> {
        let triples = parse_turtle(content)?;
        let mut dictionary = HashMap::new();
        let mut entries_by_uri = HashMap::new();
        let mut entry2dict = HashMap::new();

        let mut iter = triples.iter();
        while let Some(Triple(ref subj, ref pred, ref obj)) = iter.next() {
            if let Resource::Named(r) = subj {
                if *pred == NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") && 
                    *obj == Value::make_uri("http://www.w3.org/ns/lemon/lime#Lexicon") {
                    
                    let dict_triples = iter.clone().take_while(|t| 
                        t.1 != NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"))
                        .collect();
                    let dict = read_dictionary(release.clone(), genre.clone(), &dict_triples)?;
                    dictionary.insert(make_id(&r.uri()), dict);
                } else if *pred == NamedNode::make_uri("http://www.w3.org/ns/lemon/lime#entry") {
                    if let Value::Resource(Resource::Named(r2)) = obj {
                        entry2dict.insert(r2.uri(), make_id(&r.uri()));
                    }
                } else if *pred == NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") && 
                    is_lexical_entry_uri(&obj) {
                    let mut entry_triples : Vec<&Triple> = iter.clone().take_while(|t|
                        t.1 != NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") ||
                        !is_lexical_entry_uri(&t.2)).collect();
                    let t2 = Triple(subj.clone(), pred.clone(), obj.clone());
                    entry_triples.insert(0, &t2);
                    entries_by_uri.insert(r.uri(), add_entries(&r.uri(), 
                            &mut entry_triples)?);
                }
            }
        }

        let mut entries = HashMap::new();
        for (entry_uri, dict_uri) in entry2dict.into_iter() {
            entries.entry(dict_uri)
                .or_insert_with(|| Vec::new())
                .push(entries_by_uri.get(&entry_uri)
                    .ok_or(BackendError::OntoLex(format!("An entry <{}> is referred to as a member of a dictionary but was not found in the file", entry_uri)))?.clone());
        }

        foo(release, dictionary, entries)
}

fn is_lexical_entry_uri(value : &Value) -> bool {
    *value == Value::make_uri("http://www.w3.org/ns/lemon/ontolex#LexicalEntry") ||
    *value == Value::make_uri("http://www.w3.org/ns/lemon/ontolex#Word") ||
    *value == Value::make_uri("http://www.w3.org/ns/lemon/ontolex#MultiWordExpression") ||
    *value == Value::make_uri("http://www.w3.org/ns/lemon/ontolex#Affix")
}

fn read_dictionary(release : Release, genre : Vec<Genre>,
    triples : &Vec<&Triple>) -> Result<Dictionary, BackendError> {
    let mut source_language : Option<String> = None;
    let mut license : Option<String> = None;
    let mut creator_uris = Vec::new();
    let mut publisher_uris = Vec::new();
    let mut contributor_uris = Vec::new();
    let mut mediator_uris = Vec::new();
    let mut rights_holder_uris = Vec::new();
    let mut agent_names = HashMap::new();
    let mut agent_mboxs = HashMap::new();
    let mut agent_homepages = HashMap::new();
    let mut dc_props = HashMap::new();
    for triple in triples.iter() {
        let Triple(subj, pred, obj) = triple;
        if *pred == NamedNode::make_uri("http://www.w3.org/ns/lemon/lime#language") {
            if let Value::Literal(l) = obj {
                source_language = Some(l.string_value().to_owned())
            }
        } else if *pred == NamedNode::make_uri("http://purl.org/dc/terms/license") {
            if let Value::Resource(Resource::Named(r)) = obj {
                license = Some(r.uri())
            } else if let Value::Literal(l) = obj {
                license = Some(l.string_value().to_owned())
            }
        } else if *pred == NamedNode::make_uri("http://purl.org/dc/terms/creator") {
            if let Value::Resource(r) = obj {
                creator_uris.push(r.clone());
            }
        } else if *pred == NamedNode::make_uri("http://purl.org/dc/terms/publisher") {
            if let Value::Resource(r) = obj {
                publisher_uris.push(r.clone());
            }
         } else if *pred == NamedNode::make_uri("http://purl.org/dc/terms/contributor") {
            if let Value::Resource(r) = obj {
                contributor_uris.push(r.clone());
            }
         } else if *pred == NamedNode::make_uri("http://purl.org/dc/terms/mediator") {
            if let Value::Resource(r) = obj {
                mediator_uris.push(r.clone());
            }
         } else if *pred == NamedNode::make_uri("http://purl.org/dc/terms/rightsHolder") {
            if let Value::Resource(r) = obj {
                rights_holder_uris.push(r.clone());
            }
        } else if pred.uri().starts_with("http://purl.org/dc/terms/") {
            if let Value::Literal(l) = obj {
                dc_props.insert(pred.uri()[25..].to_owned(), l.string_value().to_owned());
            } else if let Value::Resource(Resource::Named(nn)) = obj {
                dc_props.insert(pred.uri()[25..].to_owned(), nn.uri());
            }
        } else if *pred == NamedNode::make_uri("http://xmlns.com/foaf/0.1/name") {
            if let Value::Literal(l) = obj {
                agent_names.insert(subj.clone(), l.string_value());
            }
         } else if *pred == NamedNode::make_uri("http://xmlns.com/foaf/0.1/mbox") {
            if let Value::Literal(l) = obj {
                agent_mboxs.insert(subj.clone(), l.string_value().to_owned());
            } else if let Value::Resource(Resource::Named(r2)) = obj {
                let r_uri = r2.uri();
                if r_uri.starts_with("mailto:") {
                    agent_mboxs.insert(subj.clone(), r_uri.split_at(7).1.to_owned());
                } else {
                    agent_mboxs.insert(subj.clone(), r_uri);
                }
            }
         } else if *pred == NamedNode::make_uri("http://xmlns.com/foaf/0.1/homepage") {
            if let Value::Literal(l) = obj {
                agent_homepages.insert(subj.clone(), l.string_value().to_owned());
            } else if let Value::Resource(Resource::Named(r2)) = obj {
                agent_homepages.insert(subj.clone(), r2.uri());
            }
        }
    }
    let creator = make_agents(&creator_uris, &agent_names, &agent_mboxs, &agent_homepages)?;
    let publisher = make_agents(&publisher_uris, &agent_names, &agent_mboxs, &agent_homepages)?;

    let mut dict = Dictionary::new(
        release, 
        source_language.clone().ok_or(BackendError::OntoLex("Dictionary does not have a lime:language property".to_string()))?,
        vec![source_language.unwrap().clone()], // Unwrap is safe here as line above would already have failed
        genre,
        license.ok_or(BackendError::OntoLex("Dictionary does not have a dct:license property".to_string()))?,
        creator,
        publisher
    );

    dict.contributor = make_agents(&contributor_uris, &agent_names, &agent_mboxs, &agent_homepages)?;
    dict.mediator = make_agents(&mediator_uris, &agent_names, &agent_mboxs, &agent_homepages)?;
    dict.rights_holder = make_agents(&rights_holder_uris, &agent_names, &agent_mboxs, &agent_homepages)?;

    for (prop, value) in dc_props.iter() {
        dict.set_dc_prop(prop, value);
    }

    Ok(dict)
}

fn make_agents(uris : &Vec<Resource>, agent_names : &HashMap<Resource, &str>,
    agent_mboxs : &HashMap<Resource, String>, agent_homepages : &HashMap<Resource, String>) -> Result<Vec<Agent>, BackendError> {
    let mut publisher = Vec::new();
    for publisher_uri in uris.into_iter() {
        publisher.push(Agent {
            name: (*agent_names.get(&publisher_uri).ok_or(BackendError::OntoLex(format!("A publisher <{:?}> does not have a foaf:name", publisher_uri)))?).to_owned(),
            email: agent_mboxs.get(&publisher_uri).map(|x| x.to_owned()),
            url: agent_homepages.get(&publisher_uri).map(|x| x.to_owned())
        });
    }

    Ok(publisher)

}

fn add_entries(id : &str, 
    entry_triples : &mut Vec<&Triple>) -> Result<EntryContent,BackendError> {
    let lemma = extract_lemma(id, entry_triples)?;
    let pos = extract_pos(id, entry_triples);
    let vars = extract_vars(id, entry_triples);
    let data = format_triples(entry_triples);
    Ok(EntryContent::OntoLex(make_id(id), lemma, pos, vars, data))
}

fn extract_lemma(id : &str, triples : &Vec<&Triple>) -> Result<String,BackendError> {
    triples.iter().find(|t|
        t.0 == Resource::make_uri(id) &&
        t.1 == NamedNode::make_uri("http://www.w3.org/ns/lemon/ontolex#canonicalForm"))
        .ok_or(BackendError::OntoLex("Entry has no canonical form".to_owned()))
        .and_then(|t0| {
            if let Value::Resource(ref form) = t0.2 {
                triples.iter().find(|t|
                    t.0 == *form &&
                    t.1 == NamedNode::make_uri("http://www.w3.org/ns/lemon/ontolex#writtenRep")) 
                .ok_or(BackendError::OntoLex("Canonical Form has no written rep".to_owned()))
                .and_then(|t1| {
                    match t1.2 {
                        Value::Literal(ref l) => Ok(l.string_value().to_owned()),
                        _ => Err(BackendError::OntoLex("Written rep is not a literal".to_owned()))
                    }
                })
            } else {
                Err(BackendError::OntoLex("Canonical form is not a resource".to_owned()))
            }
        })
}

fn extract_pos(id : &str, triples : &Vec<&Triple>) -> Vec<PartOfSpeech> {
    triples.iter().filter(|t|
        t.0 == Resource::make_uri(id) &&
        t.1 == NamedNode::make_uri("http://www.lexinfo.net/ontology/2.0/lexinfo#partOfSpeech"))
        .flat_map(|t| {
            if let Value::Resource(Resource::Named(ref obj)) = t.2 {
                map_pos_value(obj)
            } else {
                None
            }
        }).collect()
}

fn map_pos_value(obj : &NamedNode) -> Option<PartOfSpeech> {
    match obj.uri().as_ref() {
"http://www.lexinfo.net/ontology/2.0/lexinfo#adjective" => Some(PartOfSpeech::ADJ),
"http://www.lexinfo.net/ontology/2.0/lexinfo#adposition" => Some(PartOfSpeech::ADP),
"http://www.lexinfo.net/ontology/2.0/lexinfo#adverb" => Some(PartOfSpeech::ADV),
"http://www.lexinfo.net/ontology/2.0/lexinfo#auxiliary" => Some(PartOfSpeech::AUX),
"http://www.lexinfo.net/ontology/2.0/lexinfo#coordinatingConjunction" => Some(PartOfSpeech::CCONJ),
"http://www.lexinfo.net/ontology/2.0/lexinfo#determiner" => Some(PartOfSpeech::DET),
"http://www.lexinfo.net/ontology/2.0/lexinfo#interjection" => Some(PartOfSpeech::INTJ),
"http://www.lexinfo.net/ontology/2.0/lexinfo#commonNoun" => Some(PartOfSpeech::NOUN),
"http://www.lexinfo.net/ontology/2.0/lexinfo#numeral" => Some(PartOfSpeech::NUM),
"http://www.lexinfo.net/ontology/2.0/lexinfo#particle" => Some(PartOfSpeech::PART),
"http://www.lexinfo.net/ontology/2.0/lexinfo#properNoun" => Some(PartOfSpeech::PROPN),
"http://www.lexinfo.net/ontology/2.0/lexinfo#punctuation" => Some(PartOfSpeech::PUNCT),
"http://www.lexinfo.net/ontology/2.0/lexinfo#subordinatingConjunction" => Some(PartOfSpeech::SCONJ),
"http://www.lexinfo.net/ontology/2.0/lexinfo#symbol" => Some(PartOfSpeech::SYM),
"http://www.lexinfo.net/ontology/2.0/lexinfo#verb" => Some(PartOfSpeech::VERB),
"http://www.lexinfo.net/ontology/2.0/lexinfo#other" => Some(PartOfSpeech::X),
    _ => None
    }
}

fn extract_vars(id : &str, triples : &Vec<&Triple>) -> Vec<String> {
    triples.iter().filter(|t|
        t.0 == Resource::make_uri(id) &&
        t.1 == NamedNode::make_uri("http://www.w3.org/ns/lemon/ontolex#otherForm"))
        .flat_map(|t0| {
            if let Value::Resource(ref form) = t0.2 {
                triples.iter().find(|t|
                    t.0 == *form &&
                    t.1 == NamedNode::make_uri("http::/www.w3.org/ns/lemon/ontolex#writtenRep")) 
                .ok_or(BackendError::OntoLex("Canonical Form has no written rep".to_owned()))
                .and_then(|t1| {
                    match t1.2 {
                        Value::Literal(ref l) => Ok(l.string_value().to_owned()),
                        _ => Err(BackendError::OntoLex("Written rep is not a literal".to_owned()))
                    }
                })
            } else {
                Err(BackendError::OntoLex("Canonical form is not a resource".to_owned()))
            }
        }).collect()
}

fn format_triples(triples : &Vec<&Triple>) -> String {
    let out = String::new();
    let mut subject_pred : Option<(Resource, NamedNode)> = None;
    let prefixes = HashMap::new();
    let mut bnode_ref_count : HashMap<String, usize> = HashMap::new();

    for triple in triples.iter() {
        match triple.2 {
            Value::Resource(Resource::BlankNode(ref id)) => {
                bnode_ref_count.entry(id.to_owned()).and_modify(|e| *e += 1).or_insert(1);
            },
            _ => {}
        }
    }

    let iter = triples.iter().peekable();

    let mut state = WriteState {
        out, iter, prefixes, bnode_ref_count, 
        bnode_triples: HashMap::new(),
        indent:0
    };


    while let Some(ref triple) = state.iter.next() {
        if let Some((ref subj, ref pred)) = subject_pred {
            if *subj == triple.0 {
                if *pred == triple.1 {
                    state.out.push_str(", ");
                    write_value(&triple.2, &mut state);
                } else {
                    state.out.push_str(";\n");
                    write_pred_obj(&triple.1, &triple.2, &mut state);
                }
                subject_pred = Some((triple.0.clone(), triple.1.clone()));
            } else if let Resource::BlankNode(ref bnode) = triple.0 {
                state.bnode_triples.entry(bnode.to_owned())
                    .or_insert_with(|| Vec::new())
                    .push((**triple).clone());
            } else {
                state.out.push_str(".\n\n");
                write_triple(&triple, &mut state);
                subject_pred = Some((triple.0.clone(), triple.1.clone()));
            }
        } else {
            write_triple(&triple, &mut state);
            subject_pred = Some((triple.0.clone(), triple.1.clone()));
        }
    }
    state.out.push_str(".\n");
    state.out
}

struct WriteState<'a> {
    out : String,
    iter : Peekable<std::slice::Iter<'a, &'a Triple>>,
    prefixes : HashMap<String, Namespace>,
    bnode_ref_count : HashMap<String, usize>,
    bnode_triples : HashMap<String, Vec<Triple>>,
    indent : usize
}

fn write_value(obj : &Value, state : &mut WriteState) {
    match obj {
        Value::Literal(ref l) => write_literal(l, state),
        Value::Resource(ref r) => write_resource(r, state)
    }
}
    
fn write_literal(obj : &Literal, state : &mut WriteState) {
    match obj {
        Literal::PlainLiteral(ref s) => state.out.push_str(&format!("\"{}\" ", s.replace("\"","\\\""))),
        Literal::LangLiteral(ref s, ref l) => state.out.push_str(&format!("\"{}\"@{} ", s.replace("\"","\\\""),l)),
        Literal::TypedLiteral(ref s, ref t) => {
            state.out.push_str(&format!("\"{}\"^^", s.replace("\"", "\\\"")));
            write_named_node(t, state);
        }
    }
}

fn write_named_node(obj : &NamedNode, state : &mut WriteState) {
    if obj.uri() == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" {
        state.out.push_str("a ");
    } else {
        match obj {
            NamedNode::URIRef(uri) => state.out.push_str(&format!("<{}> ", uri)),
            NamedNode::QName(namespace, suffix) => {
                let z = state.prefixes.get(&namespace.0);
                if z == None {
                    state.prefixes.insert(namespace.0.clone(), namespace.clone());
                } else if z != Some(&namespace) {
                    panic!("Redefining a namespace");
                }
                state.out.push_str(&format!("{}:{} ", namespace.0, suffix));
            }
        }
    }
}

fn write_resource(r : &Resource, state : &mut WriteState) {
    match r {
        Resource::Named(ref nn) => write_named_node(nn, state),
        Resource::BlankNode(ref bn) => write_blank_node(bn, state)
    }
}

fn write_blank_node(bnodeid : &str, state : &mut WriteState) {
    if state.bnode_ref_count.get(bnodeid) == Some(&1) {
        state.out.push_str("[\n");
        state.indent += 1;
        let mut pred : Option<NamedNode> = None;
        let seen_triples : Vec<Triple> = state.bnode_triples.remove(bnodeid).unwrap_or_else(|| Vec::new()).clone();
        let mut iter2 = seen_triples.iter().chain(state.iter.clone().map(|x| x.clone()));
        while let Some(ref t) = iter2.next() {
            match t.0 {
                Resource::BlankNode(ref bid) if bid == bnodeid => {
                    match pred {
                        None => {},
                        Some(ref p) if *p == t.1 => {
                            state.out.push_str(",\n");
                        }
                        Some(_) => {
                            state.out.push_str(";\n");
                        }
                    }
                    write_pred_obj(&t.1, &t.2, state);
                },
                _ => {
                    break
                }
            }
            pred = Some(t.1.clone());
        }
        state.indent -= 1;
        state.out.push_str("] ");
    } else {
        state.out.push_str(&format!("_:{} ", bnodeid));
    }
}

fn write_pred_obj(pred : &NamedNode, obj : &Value, state : &mut WriteState) {
    for _ in 0..(state.indent+1) {
        state.out.push_str("  ");
    }
    write_named_node(pred, state);
    write_value(obj, state);
}

fn write_triple(triple : &Triple, state : &mut WriteState) {
    write_resource(&triple.0, state);
    write_named_node(&triple.1, state);
    write_value(&triple.2, state);
}

#[cfg(test)]
use crate::model::EDSState;
#[cfg(test)]
use crate::model::{Backend,Format};

#[test]
fn test_read_ontolex() {
    let ontolex = "@prefix lime: <http://www.w3.org/ns/lemon/lime#> .
@prefix ontolex: <http://www.w3.org/ns/lemon/ontolex#> .
@prefix dct: <http://purl.org/dc/terms/> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .
@prefix lexinfo: <http://www.lexinfo.net/ontology/2.0/lexinfo#> .

<#dictionary> a lime:Lexicon ;
    lime:language \"en\" ;
    dct:license <http://www.example.com/license> ;
    dct:creator [
        foaf:name \"Joe Bloggs\" ;
        foaf:mbox <mailto:test@example.com> ;
        foaf:homepage <http://www.example.com/>
    ] ;
    dct:publisher [
        foaf:name \"Publisher\"
    ] ;
    dct:description \"An awesome test resource\" ;
    lime:entry <#entry1>, <#entry2> .

<#entry1> a ontolex:LexicalEntry ;
    lexinfo:partOfSpeech lexinfo:commonNoun ;
    ontolex:canonicalForm [
        ontolex:writtenRep \"cat\"@en 
    ] ;
    ontolex:sense [
        skos:definition \"This is a definition\"@en
    ] .

<#entry2>  a ontolex:LexicalEntry ;
    ontolex:canonicalForm [
        ontolex:writtenRep \"dog\"@en 
    ] ;
    ontolex:sense [
        ontolex:reference <http://www.example.com/ontology>  
    ] .";

    let dictionary = parse_str(ontolex, Release::PUBLIC, vec![Genre::gen], |r,d,e| {
        Ok(BackendImpl::Mem(EDSState::new(r,d,e)))
    }).unwrap();
    assert_eq!(dictionary.dictionaries().unwrap().len(), 1);
    let dict = dictionary.about("dictionary").unwrap();
    assert_eq!(dict.release, Release::PUBLIC);
    assert_eq!(dict.source_language, "en");
    assert_eq!(dict.target_language, vec!["en"]);
    assert_eq!(dict.genre, vec![Genre::gen]);
    assert_eq!(dict.license, "http://www.example.com/license");
    assert_eq!(dict.creator, vec![Agent {
        name: "Joe Bloggs".to_owned(),
        email: Some("test@example.com".to_owned()),
        url: Some("http://www.example.com/".to_owned())
    }]);
    assert_eq!(dict.publisher, vec![Agent {
        name: "Publisher".to_owned(),
        email: None,
        url: None
    }]);
    assert_eq!(dict.description, Some("An awesome test resource".to_owned()));

    let entry_set1 = dictionary.lookup("dictionary", "cat", None, None, None, false).unwrap();
    assert_eq!(entry_set1.len(), 1);
    let ref entry1 = entry_set1[0];
    assert_eq!(entry1.release, Release::PUBLIC);
    assert_eq!(entry1.lemma, "cat");
    assert_eq!(entry1.id, "entry1");
    assert_eq!(entry1.part_of_speech, vec![PartOfSpeech::NOUN]);
    assert_eq!(entry1.formats, vec![Format::ontolex]);

    let entry_set2 = dictionary.lookup("dictionary", "dog", None, None, None, false).unwrap();
    assert_eq!(entry_set2.len(), 1);
    let ref entry2 = entry_set2[0];
    assert_eq!(entry2.release, Release::PUBLIC);
    assert_eq!(entry2.lemma, "dog");
    assert_eq!(entry2.id, "entry2");
    assert_eq!(entry2.part_of_speech, vec![]);
    assert_eq!(entry2.formats, vec![Format::ontolex]);

    let entry1_ontolex = dictionary.entry_ontolex("dictionary", "entry1").unwrap();

    assert_eq!(entry1_ontolex, "<#entry1> a ontolex:LexicalEntry ;
  lexinfo:partOfSpeech lexinfo:commonNoun ;
  ontolex:canonicalForm [
    ontolex:writtenRep \"cat\"@en ] ;
  ontolex:sense [
    skos:definition \"This is a definition\"@en ] .
");




}



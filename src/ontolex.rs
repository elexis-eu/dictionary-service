use std::io::Read;
use crate::model::{Release, Genre, Dictionary, EntryContent, PartOfSpeech,BackendError};
use crate::BackendImpl;
use std::collections::HashMap;
use std::iter::Peekable;
use crate::rdf::turtle::parse_turtle;
use crate::rdf::model::{NamedNode,Value,Resource,Triple,Namespace,Literal};

pub fn parse<R : Read, F>(mut input : R, id : &str, release : Release,
    genre : Vec<Genre>, foo : F) -> Result<BackendImpl,BackendError>
    where F : FnOnce(Release, HashMap<String, Dictionary>, HashMap<String, Vec<EntryContent>>) -> Result<BackendImpl,BackendError> {
        let mut content = String::new();
        input.read_to_string(&mut content)?;

        let triples = parse_turtle(&content)?;
        let mut entry_triple = Vec::new();
        let mut dictionary = HashMap::new();
        let mut entries = HashMap::new();
        let mut current_dictionary = String::from(id);


        for triple in triples.iter() {
            if triple.1 == NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") && 
                triple.2 == Value::make_uri("http://www.w3.org/ns/lemon/lime#Lexicon") {
                if let Resource::Named(ref subj) = triple.0 {
                    current_dictionary = subj.uri()
                }
            } else if triple.1 == NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") && 
                is_lexical_entry_uri(&triple.2) {
                add_entries(&mut entries, &current_dictionary, &mut entry_triple)?;
            }
        }

        foo(release, dictionary, entries)
}

fn is_lexical_entry_uri(value : &Value) -> bool {
    *value == Value::make_uri("http://www.w3.org/ns/lemon/ontolex#LexicalEntry") ||
    *value == Value::make_uri("http://www.w3.org/ns/lemon/ontolex#Word") ||
    *value == Value::make_uri("http://www.w3.org/ns/lemon/ontolex#MultiWordExpression") ||
    *value == Value::make_uri("http://www.w3.org/ns/lemon/ontolex#Affix")
}

fn add_entries(entries : &mut HashMap<String, Vec<EntryContent>>, dict_id : &str,
    entry_triples : &mut Vec<Triple>) -> Result<(),BackendError> {
    let id = extract_id(entry_triples)?;
    let lemma = extract_lemma(&id, entry_triples)?;
    let pos = extract_pos(&id, entry_triples);
    let vars = extract_vars(&id, entry_triples);
    let data = format_triples(entry_triples);
    entries.entry(dict_id.to_string())
        .or_insert_with(|| Vec::new())
        .push(EntryContent::OntoLex(id, lemma, pos, vars, data));
    entry_triples.clear();
    Ok(())
}

fn extract_id(triples : &Vec<Triple>) -> Result<String,BackendError> {
    triples.iter().find(|t|
        t.0.is_uri() &&
        t.1 == NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type") && 
        is_lexical_entry_uri(&t.2))
        .ok_or(BackendError::OntoLex("No URI for triple".to_owned()))
        .and_then(|t| {
            if let Resource::Named(ref subj) = t.0 {
                Ok(subj.uri())
            } else {
                Err(BackendError::OntoLex("Blank node as subject of lexical entry".to_owned()))
            }
        })
}

fn extract_lemma(id : &str, triples : &Vec<Triple>) -> Result<String,BackendError> {
    triples.iter().find(|t|
        t.0 == Resource::make_uri(id) &&
        t.1 == NamedNode::make_uri("http://www.w3.org/ns/lemon/ontolex#canonicalForm"))
        .ok_or(BackendError::OntoLex("Entry has no canonical form".to_owned()))
        .and_then(|t0| {
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
        })
}

fn extract_pos(id : &str, triples : &Vec<Triple>) -> Vec<PartOfSpeech> {
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

fn extract_vars(id : &str, triples : &Vec<Triple>) -> Vec<String> {
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

fn format_triples(triples : &Vec<Triple>) -> String {
    let mut out = String::new();
    let mut subject_pred : Option<(Resource, NamedNode)> = None;
    let mut prefixes = HashMap::new();
    let mut bnode_ref_count : HashMap<String, usize> = HashMap::new();

    for triple in triples.iter() {
        match triple.2 {
            Value::Resource(ref r) if r.is_bnode() => {
                bnode_ref_count.entry(triple.2.to_string()).and_modify(|e| *e += 1).or_insert(1);
            },
            _ => {}
        }
    }

    let iter = triples.iter().peekable();

    let mut state = WriteState {
        out, iter, prefixes, bnode_ref_count, indent:0
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
            } else {
                state.out.push_str(".\n\n");
                write_triple(&triple, &mut state);
            }
        } else {
            write_triple(&triple, &mut state);
        }
    }
    state.out
}

struct WriteState<'a> {
    out : String,
    iter : Peekable<std::slice::Iter<'a, Triple>>,
    prefixes : HashMap<String, Namespace>,
    bnode_ref_count : HashMap<String, usize>,
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
        let mut iter2 = state.iter.clone();
        while let Some(ref t) = iter2.next() {
            match pred {
                None => {},
                Some(ref p) if *p == t.1 => {
                    state.out.push_str(",\n");
                }
                Some(_) => {
                    state.out.push_str(";\n");
                }
            }
            match t.0 {
                Resource::BlankNode(ref bid) if bid == bnodeid => {
                    if let Some(ref t) = state.iter.next() {
                        write_pred_obj(&t.1, &t.2, state);
                    }
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
    for i in 0..state.indent {
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



//fn format_triples(triples : &mut Vec<Triple>) -> String {
//    let mut data = String::new();
//    triples.sort();
//    let mut subject_pred : Option<(Resource,NamedNode)> = None;
//    let mut indent = 0;
//
//    let mut bnode_refs : HashMap<String, usize> = HashMap::new();
//
//    for triple in triples.iter() {
//        match triple.2 {
//            Value::Resource(ref r) if r.is_bnode() => {
//                bnode_refs.entry(triple.2.to_string()).and_modify(|e| *e += 1).or_insert(1);
//            },
//            _ => {}
//        }
//    }
//
//    let btriples : Vec<Triple> = triples.drain_filter(|t| {
//        t.0.is_bnode() && bnode_refs.get(&t.0.to_string()) == Some(&1)
//    }).collect();
//
//    while let Some(t) = triples.pop() {
//        if let Some((subj,pred)) = subject_pred {
//            if subj == triple.0 {
//                if pred == triple.1 {
//                    data.push_str(", ");
//                    write_obj(&triple.2, &mut data, &mut btriples, &bnode_refs);
//                } else {
//                    data.push_str(";\n");
//                    indent = write_pred_obj(&triple.1, &triple.2, &mut data, indent);
//                }
//            } else {
//                data.push_str(".\n\n");
//                indent = write_triple(&triple, &mut data, indent)
//            }
//        } else {
//            indent = write_triple(triple, &mut data, indent);
//        }
//        subject_pred = Some((triple.0.clone(), triple.1.clone()))
//    }
//        
//    panic!("TODO")
//}
//
//fn write_triple(triple : &Triple, out : &mut String, indent : u32, prefixes : &mut HashMap<String, Namespace>) -> u32 {
//    panic!("TODO")
//}
//
//fn write_pred_obj(pred : &NamedNode, obj : &Value, out : &mut String, prefixes : &mut HashMap<String, Namespace>, btriples : &mut Vec<Triple>, indent : usize) {
//    panic!("TODO")
//}
//
//fn write_obj(obj : &Value, out : &mut String, prefixes : &mut HashMap<String, Namespace>,
//    btriples : &mut Vec<Triple>, indent : usize) {
//    match obj {
//        Value::Literal(ref l) => write_literal(l, out, prefixes),
//        Value::Resource(ref r) => write_resource(r, out, prefixes)
//    }
//    panic!("TODO")
//}
//
//fn write_literal(obj : &Literal, out : &mut String, prefixes : &mut HashMap<String, Namespace>) {
//    match obj {
//        Literal::PlainLiteral(ref s) => out.push_str(&format!("\"{}\" ", s.replace("\"","\\\""))),
//        Literal::LangLiteral(ref s, ref l) => out.push_str(&format!("\"{}\"@{} ", s.replace("\"","\\\""),l)),
//        Literal::TypedLiteral(ref s, ref t) => {
//            out.push_str(&format!("\"{}\"^^", s.replace("\"", "\\\"")));
//            write_named_node(t, out, prefixes);
//        }
//    }
//}
//
//fn write_resource(r : &Resource, out : &mut String, prefixes : &mut HashMap<String, Namespace>, btriples : &mut Vec<Triple>, indent : usize) {
//    match r {
//        Resource::BlankNode(ref bn) => write_bnode(bn, out, prefixes, btriples, indent),
//        Resource::Named(ref nn) => write_named_node(nn, out, prefixes)
//    }
//}
//
//fn write_named_node(obj : &NamedNode, out : &mut String, prefixes : &mut HashMap<String, Namespace>) {
//    match obj {
//        NamedNode::URIRef(uri) => out.push_str(&format!("<{}> ", uri)),
//        NamedNode::QName(namespace, suffix) => {
//           let z = prefixes.get(&namespace.0);
//           if z == None {
//               prefixes.insert(namespace.0.clone(), namespace.clone());
//           } else if z != Some(&namespace.1) {
//               panic!("Redefining a namespace");
//           }
//           out.push_str(&format!("{}:{} ", namespace.0, suffix));
//        }
//    }
//}
//
//fn write_bnode(bnode_id : &str, out : &mut String, prefixes : &mut HashMap<String, Namespace>,
//    btriples : &mut Vec<Triple>, indent : usize) {
//    let bn = Resource::BlankNode(bnode_id.to_string());
//    let rel_triples : Vec<Triple> = btriples.drain_filter(|t| t.0 == bn).collect();
//    if rel_triples.is_empty() {
//        out.push_str(&format!("_:{} ", bnode_id));
//    } else {
//        out.push_str("[\n");
//        for t in rel_triples.into_iter() {
//            write_pred_obj(t.1, t.2, out, prefixes, btriples, indent + 1);
//        }
//        out.push_str("] ");
//    }
//}

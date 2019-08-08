use std::io::Read;
use crate::model::{Agent, Release, Genre, Format, Entry, Dictionary, PartOfSpeech, EntryContent};
use crate::BackendImpl;

use xml::reader::{EventReader, XmlEvent};
use xml::name::OwnedName;
use xml::attribute::OwnedAttribute;
use xml::namespace::Namespace;
use xml::escape::escape_str_attribute;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

pub fn parse<R : Read,F>(input : R, id : &str, release : Release,
                   genre : Vec<Genre>, foo : F) -> BackendImpl 
    where F : FnOnce(Release, HashMap<String,Dictionary>, HashMap<String, Vec<EntryContent>>) -> BackendImpl {
    let parser = EventReader::new(input);

    let mut target_language = Vec::new();
    let mut licence = None;
    let mut creators = Vec::new();
    let mut publishers = Vec::new();
    let mut creator = Agent::new();
    let mut publisher = Agent::new();
    
    let mut state = State::Empty;

    let mut lemma = String::new();
    let mut variants = Vec::new();
    let mut variant = String::new();
    let mut entry_id = None;
    let mut part_of_speech = Vec::new();
    let mut content = String::new();
    let mut language = None;

    let mut pos_string = String::new();

    let mut entries = Vec::new();
    let mut anon_count = 0u32;

    for e in parser {
        match e { 
            Ok(XmlEvent::StartElement { name, attributes, namespace }) => {
                if name.local_name == "author" {
                    state = State::Author;
                } else if name.local_name == "publisher" {
                    state = State::Publisher;
                } else if name.local_name == "licence" {
                    match attributes.iter().find(|x| x.name.local_name == "target") {
                        Some(attr) => {
                            licence = Some(attr.value.to_string());
                        },
                        None => {
                            eprintln!("<licence> without target");
                        }
                    }
                } else if name.local_name == "entry" {
                    state = State::Entry;
                    match attributes.iter().find(|x| x.name.local_name == "lang") {
                        Some(attr) => {
                            language = Some(attr.value.to_string());
                        },
                        None => {}
                    };
                    match attributes.iter().find(|x| x.name.local_name == "id") {
                        Some(attr) => {
                            entry_id = Some(attr.value.to_string());
                        },
                        None => {}
                    };
                    content.clear();
                    extend_content_tag(&mut content, name, attributes, namespace);
                } else if state == State::Entry || state == State::Lemma {
                    if name.local_name == "form" &&
                        attributes.iter().any(|x| x.name.local_name == "type" &&
                                                  x.value == "lemma") {
                        state = State::Lemma
                    }
                    if name.local_name == "form" &&
                        attributes.iter().any(|x| x.name.local_name == "type" &&
                                                  x.value == "variant") {
                        variant.clear();
                        state = State::Variant
                    }
                    if name.local_name == "pos" || name.local_name == "gram"
                        && attributes.iter().any(|x| x.name.local_name == "type" &&
                                                 x.value == "pos") {
                        state = State::Pos;
                        part_of_speech.clear();
                        if let Some(norm) = attributes.iter().find(|x| x.name.local_name == "norm") {
                            if let Ok(p) = PartOfSpeech::from_str(&norm.value) {
                                part_of_speech = vec![p];
                            } else {
                                eprintln!("Bad normalization: {}", norm.value);
                            }
                        }
                        pos_string = String::new();
                    }

                    match attributes.iter().find(|x| x.name.local_name == "lang") {
                        Some(attr) => { target_language.push(attr.value.to_string()); },
                        None => {}
                    };
                    extend_content_tag(&mut content, name, attributes, namespace);
                }
            },
            Ok(XmlEvent::EndElement { name }) => {
                if name.local_name == "author" {
                    creators.push(creator);
                    creator = Agent::new();
                    state = State::Empty;
                } else if name.local_name == "publisher" {
                    publishers.push(publisher);
                    publisher = Agent::new();
                    state = State::Empty;
                } else if name.local_name == "entry" {
                    extend_content_endtag(&mut content, name);
                    let lang = match language {
                        Some(ref language) => language,
                        None => {
                            eprintln!("No language on entry");
                            "und"
                        }
                    };
                    let id = match entry_id {
                        Some(ref entry_id) => entry_id.to_string(),
                        None => {
                            eprintln!("No id on entry");
                            anon_count += 1;
                            format!("unidentified_entry_{}", anon_count)
                        }
                    };
                    if part_of_speech.is_empty() {
                        part_of_speech.push(PartOfSpeech::X)
                    }
                    if lemma == "" {
                        lemma.push_str("<Empty lemma>");
                    }
                    entries.push((
                            lang.to_string(),
                            Entry::new(
                                release.clone(), 
                                lemma.to_string(),
                                id,
                                part_of_speech.clone(),
                                vec![Format::tei]),
                            variants.clone(),
                            content.clone()));
                    lemma = String::new();
                    entry_id = None;
                    part_of_speech = Vec::new();
                    content = String::new();
                    language = None;
                    variants.clear();
                } else if name.local_name == "form" && state == State::Lemma {
                    extend_content_endtag(&mut content, name);
                    state = State::Entry;
                } else if name.local_name == "form" && state == State::Lemma {
                    extend_content_endtag(&mut content, name);
                    variants.push(variant.clone());
                    state = State::Entry;
                } else if (name.local_name == "gram"  || name.local_name == "pos") 
                    && state == State::Pos {
                    extend_content_endtag(&mut content, name);
                    if part_of_speech.is_empty() { // we did not get a pos from the normalization
                        part_of_speech.push(convert_pos(&pos_string));
                    }
                    state = State::Entry;
                } else if state == State::Entry || state == State::Lemma || state == State::Pos {
                    extend_content_endtag(&mut content, name);
                }
            },
            Ok(XmlEvent::Characters(s)) => {
                if state == State::Author {
                    creator.name.push_str(&s);
                } else if state == State::Publisher {
                    publisher.name.push_str(&s);
                } else if state == State::Entry {
                    content.push_str(&s);
                } else if state == State::Lemma {
                    content.push_str(&s);
                    lemma.push_str(&s);
                } else if state == State::Variant {
                    content.push_str(&s);
                    variant.push_str(&s);
                } else if state == State::Pos {
                    content.push_str(&s);
                    pos_string.push_str(&s);
                }
            },
            Ok(XmlEvent::Whitespace(s)) => {
                content.push_str(&s);
            },
            Err(e) => {
                eprintln!("Failed to load TEI file: {:?}", e);
            },
            Ok(_) => {}
        }
    }

    let mut dictionaries = HashMap::new();
    let mut dict_entries = HashMap::new();
    let src_langs = entries.iter().map(|x| x.0.clone()).collect::<HashSet<String>>();
    if src_langs.len() > 1 {
        for src_lang in src_langs {
            let dict_id = format!("{}-{}", id, src_lang);
           dictionaries.insert(dict_id.clone(),
            Dictionary::new(
                release.clone(), src_lang.to_string(),
                target_language.clone(),
                genre.clone(), licence.clone().unwrap_or("unknown".to_string()),
                creators.clone(), publishers.clone()));
           build_entries(&dict_id, &mut dict_entries, &entries, &src_lang);
        }
    } else if src_langs.len() == 1 {
        let src_lang = src_langs.iter().next().unwrap();
        dictionaries.insert(id.to_string(),
        Dictionary::new(
            release.clone(), src_lang.to_string(),
            target_language,
            genre.clone(), licence.unwrap_or("unknown".to_string()),
            creators.clone(), publishers.clone()));
       build_entries(&id, &mut dict_entries, &entries, &src_lang);
    }
 
    foo(release, dictionaries, dict_entries)
}

fn build_entries(dict_id : &str,
    dict_entries : &mut HashMap<String, Vec<EntryContent>>,
    entries : &Vec<(String, Entry, Vec<String>, String)>,
    lang : &str) {

    for entry in entries.iter() {
        if entry.0 == lang {

            dict_entries.entry(dict_id.to_string())
                .or_insert_with(|| Vec::new())
                .push(EntryContent::Tei(
                        entry.1.id.to_string(),
                        entry.1.lemma.to_string(),
                        entry.1.part_of_speech.clone(),
                        entry.2.clone(),
                        detab_content(&entry.3)));
        }
    }

}

fn extend_content_tag(content : &mut String, name : OwnedName, 
                      attributes : Vec<OwnedAttribute>, _namespace : Namespace) {
    content.push_str("<");
    match name.prefix {
        Some(n) => {
            content.push_str(&n);
            content.push_str(":");
        },
        None => {}
    };
    content.push_str(&name.local_name);
    for attr in attributes {
        content.push_str(" ");
        match attr.name.prefix {
            Some(n) => {
                content.push_str(&n);
                content.push_str(":");
            },
            None => {}
        }
        content.push_str(&attr.name.local_name);
        content.push_str("=\"");
        content.push_str(&escape_str_attribute(&attr.value));
        content.push_str("\"");
    }
    content.push_str(">");
}

fn extend_content_endtag(content : &mut String, name : OwnedName) {
    content.push_str("</");
    match name.prefix {
        Some(n) => {
            content.push_str(&n);
            content.push_str(":");
        },
        None => {}
    };
    content.push_str(&name.local_name);
    content.push_str(">");
}

fn detab_content(content : &str) -> String {
    let mut indent = std::usize::MAX;
    let mut first = true;
    for line in content.split("\n") {
        if first {
            first = false;
        } else if line != "" {
            let i = line.chars().take_while(|c| *c == ' ').count();
            indent = std::cmp::min(i, indent);
        }
    }
    if indent == 0 || indent == std::usize::MAX {
        content.to_string()
    } else {
        let mut detabbed = String::new();
        first = true;
        for line in content.split("\n") {
            if first || line == "" {
                first = false;
                detabbed.push_str(&line);
                detabbed.push_str("\n");
            } else {
                detabbed.push_str(&line[indent..]);
                detabbed.push_str("\n");
            }
        }
        detabbed
    }
}

fn convert_pos(pos : &str) -> PartOfSpeech {
    match pos {
        "adjective" => PartOfSpeech::ADJ,
        "adposition" => PartOfSpeech::ADP,
        "adverb" => PartOfSpeech::ADV,
        "auxiliary" => PartOfSpeech::AUX,
        "coordinatingConjunction" => PartOfSpeech::CCONJ,
        "determiner" => PartOfSpeech::DET,
        "interjection" => PartOfSpeech::INTJ,
        "commonNoun" => PartOfSpeech::NOUN,
        "numeral" => PartOfSpeech::NUM,
        "particle" => PartOfSpeech::PART,
        "properNoun" => PartOfSpeech::PROPN,
        "punctuation" => PartOfSpeech::PUNCT,
        "subordinatingConjunction" => PartOfSpeech::SCONJ,
        "symbol" => PartOfSpeech::SYM,
        "verb" => PartOfSpeech::VERB,
        _ => PartOfSpeech::X
    }
}


// What the parse is currently doing
#[derive(Debug,PartialEq)]
enum State {
    Empty,
    Author,
    Publisher,
    Entry,
    Lemma,
    Pos,
    Variant
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EDSState;

    #[test]
    fn test_load_tei() {
        let doc = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<?xml-model href=\"http://www.tei-c.org/release/xml/tei/custom/schema/relaxng/tei_all.rng\" type=\"application/xml\"
	schematypens=\"http://purl.oclc.org/dsdl/schematron\"?>
<?xml-model href=\"../../Schemas/TEILex0/out/TEILex0-ODD.rng\" type=\"application/xml\" schematypens=\"http://relaxng.org/ns/structure/1.0\"?>
<TEI xmlns=\"http://www.tei-c.org/ns/1.0\">
   <teiHeader>
      <fileDesc>
         <titleStmt>
            <title>Sample Etymological Phenomena for TEI Lex0 Etymology Paper</title>
            <author>Jack Bowers</author>
         </titleStmt>
         <publicationStmt>
            <publisher>Publication Information</publisher>
            <availability>
                <licence target=\"http://www.example.com/licence\"/>
            </availability>
         </publicationStmt>
         <sourceDesc>
            <p>Information about the source</p>
         </sourceDesc>
      </fileDesc>
      <encodingDesc>
         <ab>Examples are encoded in TEI-Lex0 Etymology format</ab>
      </encodingDesc>
   </teiHeader>
   <text>
      <body>
         <!-- Cognate Set -->
         <entry xml:lang=\"en\" xml:id=\"girl-en\">
            <form type=\"lemma\">
               <orth>girl</orth>
            </form>
            <etym>
               <cit type=\"cognateSet\">
                  <lbl>Cf.</lbl>
                  <cit type=\"cognate\">
                     <lang>NFries.</lang>
                     <form>
                        <orth xml:lang=\"ffr\">gör</orth>
                     </form>
                     <def>a girl</def>
                  </cit>
                  <cit type=\"cognate\" xml:lang=\"nds-x-pom\"><!-- Pomeran. german dialect variety of low german 
                     a more specific variety \"East Pomeranian\"
                     has no ISO tag but a Glottolog one: east2293 -->
                     <lang>Pomeran.</lang>
                     <form>
                        <orth>goer</orth>
                     </form>
                     <def>a child</def>
                  </cit>
                  <cit type=\"cognate\">
                     <lang>O. Low. G.</lang>
                     <form>
                        <orth xml:lang=\"nds\">gör</orth>
                     </form>
                     <def>a child</def>
                     <bibl>Bremen Wörterbuch, ii. 528</bibl>
                  </cit>
                  <cit type=\"cognate\">
                     <lang>Swiss</lang>
                     <form type=\"variant\">
                        <orth xml:lang=\"gsw\">gurre</orth>
                     </form>
                     <form type=\"variant\">
                        <orth xml:lang=\"gsw\">gurrli</orth>
                     </form>
                     <def>a depreciatory term for a girl</def>
                     <bibl>Sanders, G. Dict. i. 609. 641</bibl>
                  </cit>
                  <cit type=\"cognate\">
                     <lang>Norw.</lang>
                     <form>
                        <orth xml:lang=\"nor\">gorre</orth>
                     </form>
                     <def>a small child</def>
                     <bibl>(Aasen)</bibl>
                  </cit>
                  <cit type=\"cognate\">
                     <lang>Swed. dial.</lang>
                     <form type=\"variant\">
                        <orth xml:lang=\"swe-x-dia\">gårrä</orth>
                     </form>
                     <form type=\"variant\">
                        <orth xml:lang=\"swe-x-dia\">gurre</orth>
                     </form>
                     <def>a small child</def>
                  </cit>
               </cit>
            </etym>
         </entry> 
      </body>
    </text>
</TEI>";

        parse(doc.as_bytes(), "test-dict", Release::PUBLIC, Vec::new(), |r,d,e| {
            BackendImpl::Mem(EDSState::new(r,d,e)) 
        });
    }

    #[test]
    fn test_example() {
        let x : &[u8] = include_bytes!("../examples/example-tei.xml");
        let _state = parse(x, "exmaple-tei", Release::PUBLIC, Vec::new(), |r,d,e| {
            BackendImpl::Mem(EDSState::new(r,d,e)) 
        });
        //assert_eq!(state.dictionaries.lock().unwrap().len(), 1);
        //assert!(state.entries_lemmas.lock().unwrap().contains_key("exmaple-tei"));
        //assert!(state.entries_lemmas.lock().unwrap().get("exmaple-tei").unwrap().contains_key("girl"));

    }

}

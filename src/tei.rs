use std::io::Read;
use crate::model::{EDSState, Agent, Release, Genre, Format, Entry, Dictionary, PartOfSpeech};

use xml::reader::{EventReader, XmlEvent};
use xml::name::OwnedName;
use xml::attribute::OwnedAttribute;
use xml::namespace::Namespace;
use xml::escape::escape_str_attribute;

use std::collections::{HashMap, HashSet};

fn parse<R : Read>(input : R, id : &str, release : Release,
                   genre : Vec<Genre>) -> EDSState {
    let parser = EventReader::new(input);

    let mut target_language = Vec::new();
    let mut licence = None;
    let mut creators = Vec::new();
    let mut publishers = Vec::new();
    let mut creator = Agent::new();
    let mut publisher = Agent::new();
    
    let mut state = State::Empty;

    let mut lemma = String::new();
    let mut entry_id = None;
    let mut part_of_speech = Vec::new();
    let mut content = String::new();
    let mut language = None;

    let mut pos_string = String::new();

    let mut entries = Vec::new();

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
                    extend_content_tag(&mut content, name, attributes, namespace);
                } else if state == State::Entry || state == State::Lemma {
                    if name.local_name == "form" &&
                        attributes.iter().any(|x| x.name.local_name == "type" &&
                                                  x.value == "lemma") {
                        state = State::Lemma
                    }
                    if name.local_name == "pos" || name.local_name == "gramGrp"
                        && attributes.iter().any(|x| x.name.local_name == "type" &&
                                                 x.value == "pos") {
                        state = State::Pos;
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
                    match language {
                        Some(ref language) => {
                            match entry_id {
                                Some(ref entry_id) => {
                                    entries.push((
                                            language.clone(),
                                            Entry::new(
                                                release.clone(), 
                                                lemma.to_string(),
                                                entry_id.to_string(),
                                                part_of_speech.clone(),
                                                vec![Format::tei]),
                                            content.clone()));
                                },
                                None => {
                                    eprintln!("No id on entry");
                                }
                            }
                        },
                        None => {
                            eprintln!("No language on entry");
                        }
                    };
                    lemma = String::new();
                    entry_id = None;
                    part_of_speech = Vec::new();
                    content = String::new();
                    language = None;
                } else if name.local_name == "form" && state == State::Lemma {
                    extend_content_endtag(&mut content, name);
                    state = State::Entry;
                } else if (name.local_name == "gramGrp"  || name.local_name == "pos") 
                    && state == State::Pos {
                    extend_content_endtag(&mut content, name);
                    part_of_speech.push(convert_pos(&pos_string));
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
                } else if state == State::Pos {
                    content.push_str(&s);
                    pos_string.push_str(&s);
                }
            },
            Err(e) => {
                eprintln!("Failed to load TEI file: {:?}", e);
            },
            Ok(_) => {}
        }
    }

    let mut dictionaries = HashMap::new();
    let dict_entries = HashMap::new();
    let src_langs = entries.iter().map(|x| x.0.clone()).collect::<HashSet<String>>();
    if src_langs.len() > 1 {
        for src_lang in src_langs {
           dictionaries.insert(format!("{}-{}", id, src_lang),
            Dictionary::new(
                release.clone(), src_lang.to_string(),
                target_language.clone(),
                genre.clone(), licence.clone().unwrap_or("unknown".to_string()),
                creators.clone(), publishers.clone()));
        }
    } else if src_langs.len() == 1 {
        let src_lang = src_langs.iter().next().unwrap();
        dictionaries.insert(id.to_string(),
        Dictionary::new(
            release.clone(), src_lang.to_string(),
            target_language,
            genre.clone(), licence.unwrap_or("unknown".to_string()),
            creators.clone(), publishers.clone()));
    }
 
    EDSState::new(dictionaries, dict_entries)
}

fn extend_content_tag(content : &mut String, name : OwnedName, 
                      attributes : Vec<OwnedAttribute>, namespace : Namespace) {
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
    content.push_str(">");
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
    Pos
}

#[cfg(test)]
mod tests {
    use super::*;

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

        parse(doc.as_bytes(), "test-dict", Release::PUBLIC, Vec::new());
    }

    #[test]
    fn test_example() {
        let x : &[u8] = include_bytes!("../examples/example-tei.xml");
        parse(x, "exmaple-tei", Release::PUBLIC, Vec::new());
    }
}

use std::path::Path;
use std::io::BufReader;
use std::fs::File;
use crate::state::{EDSState, Agent, Release};

use xml::reader::{EventReader, XmlEvent};

fn parse<P : AsRef<Path>>(file : P, id : String, release : Release) -> EDSState {
    let file = File::open("file.xml").unwrap();
    let file = BufReader::new(file);

    let parser = EventReader::new(file);

//    let mut source_language = None;
//    let mut target_language = Vec::new();
//    let mut genre = Vec::new();
//    let mut license = None;
    let mut creators = Vec::new();
    let mut publishers = Vec::new();
    let mut creator = Agent::new();
    let mut publisher = Agent::new();
    
    let mut state = State::Empty;

    for e in parser {
        match e { 
            Ok(XmlEvent::StartElement { name, attributes, namespace }) => {
                if name.local_name == "author" {
                    state = State::Author;
                } else if name.local_name == "publisher" {
                    state = State::Publisher;
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
                }
            },
            Ok(XmlEvent::Characters(s)) => {
                if state == State::Author {
                    creator.name.push_str(&s);
                } else if state == State::Publisher {
                    publisher.name.push_str(&s);
                }
            },
            Err(e) => {
                eprintln!("Failed to load TEI file");
            },
            Ok(_) => {}
        }
    }
    panic!("TODO")
}

// What the parse is currently doing
#[derive(Debug,PartialEq)]
enum State {
    Empty,
    Author,
    Publisher
}

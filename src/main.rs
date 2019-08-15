#![feature(drain_filter)]
extern crate gotham;
extern crate http;
extern crate hyper;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate gotham_derive;
extern crate clap;
extern crate xml;
extern crate pest;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate quick_error;

mod rest;
mod model;
mod tei;
mod rdf;
mod sqlite;
mod ontolex;
mod config;

use gotham::state::State;
use gotham::router::Router;
use gotham::router::builder::*;
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single::single_pipeline;
use gotham::pipeline::single_middleware;

use http::Method;
use http::{Response, StatusCode};

use mime::Mime;

use hyper::Body;

use clap::{App, Arg, SubCommand, ArgMatches};

use std::fs::File;
use std::collections::HashMap;
use std::str::FromStr;
use std::error::Error;

use crate::model::{EDSState, Dictionary, JsonEntry, PartOfSpeech, EntryContent, BackendError, Entry};
use crate::sqlite::RusqliteState;
use crate::config::Config;

fn router(model : BackendImpl) -> Router {
    let middleware = StateMiddleware::new(model);
    let pipeline = single_middleware(middleware);
    let (chain, pipelines) = single_pipeline(pipeline);

    build_router(chain, pipelines, |route| {
        route.request(vec![Method::GET, Method::HEAD], "/").to(index);
        route.request(vec![Method::GET], "/dictionaries").to(rest::dictionaries);
        route.get("/about/:dictionary")
            .with_path_extractor::<AboutParams>()
            .to(rest::about);
        route.get("/list/:dictionary")
            .with_path_extractor::<ListPathParams>()
            .with_query_string_extractor::<ListQueryParams>()
            .to(rest::list);
        route.get("/lemma/:dictionary/:headword")
            .with_path_extractor::<LookupPathParams>()
            .with_query_string_extractor::<LookupQueryParams>()
            .to(rest::lookup);
        route.get("/json/:dictionary/:id")
            .with_path_extractor::<EntryPathParams>()
            .to(rest::entry_json);
        route.get("/ontolex/:dictionary/:id")
            .with_path_extractor::<EntryPathParams>()
            .to(rest::entry_ontolex);
        route.get("/tei/:dictionary/:id")
            .with_path_extractor::<EntryPathParams>()
            .to(rest::entry_tei);
        route.get("/img/logo.jpg")
            .to(logo);
    })
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct AboutParams {
    dictionary: String,
}
#[derive(Deserialize, StateData, StaticResponseExtender)]
struct ListPathParams {
    dictionary : String
}
#[derive(Deserialize, StateData, StaticResponseExtender)]
struct ListQueryParams {
    offset : Option<usize>,
    limit : Option<usize>
}
#[derive(Deserialize, StateData, StaticResponseExtender)]
struct LookupPathParams {
    dictionary : String,
    headword : String
}
#[derive(Deserialize, StateData, StaticResponseExtender)]
#[serde(rename_all = "camelCase")]
struct LookupQueryParams {
    part_of_speech : Option<PartOfSpeech>,
    limit : Option<usize>,
    offset : Option<usize>,
    inflected : Option<bool>
}
#[derive(Deserialize, StateData, StaticResponseExtender)]
struct EntryPathParams {
    dictionary : String,
    id : String
}


pub fn logo(state : State) -> (State, (Mime, Body)) {
    let mut v = Vec::new();
    v.extend(include_bytes!("img/logo.jpg").iter());
    let res = (mime::IMAGE_JPEG, Body::from(v));
    (state, res)
}

pub fn index(state : State) -> (State, Response<Body>) {
    (state, Response::builder()
        .header("Content-Type", "text/html")
        .status(StatusCode::OK)
        .body(Body::from(include_str!("html/index.html"))).unwrap())
}

fn main() {
    let mut app = App::new("ELEXIS Dictionary Service")
        .version("0.1")
        .author("John P. McCrae <john@mccr.ae>")                    
        .about("Server for hosting dictionaries so they may be accessed by the Dictionary Matrix")
        .subcommand(SubCommand::with_name("load")
            .about("Load data into the database")
            .arg(Arg::with_name("data")
                .help("The data to host")
                .required(true)
                .index(1))
            .arg(Arg::with_name("format")
                .help("The format of the input")
                .value_name("json|ttl|tei")
                .short("f")
                .long("format")
                .takes_value(true))
            .arg(Arg::with_name("release")
                .help("The release level of the resource")
                .takes_value(true)
                .long("release")
                .value_name("PUBLIC|NONCOMMERCIAL|RESEARCH|PRIVATE"))
            .arg(Arg::with_name("genre")
                .help("The genre(s) of the dataset (comma separated)")
                .takes_value(true)
                .use_delimiter(true)
                .long("genre")
                .value_name("gen|lrn|ety|spe|his|ort|trm"))
            .arg(Arg::with_name("id")
                .help("The identifier of the dataset")
                .long("id")
                .takes_value(true))
            .arg(Arg::with_name("no_sql")
                .help("Do not use SQLite (all data is temporary and session only)")
                .long("no-sql"))                        
            .arg(Arg::with_name("config")
                .help("Configuration to help with mapping")
                .short("c")
                .long("config")
                .takes_value(true))
            .arg(Arg::with_name("db_path")
                .help("The path to use for the database (Default: eds.db)")
                .long("db-path")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("start")
            .about("Start the server")
            .arg(Arg::with_name("port")
                .help("The port to start the server on")
                .short("p")
                .long("port")
                .required(false)
                .takes_value(true))
            .arg(Arg::with_name("data")
                .help("Also load a single data file")
                .short("d")
                .long("data")
                .required(false)
                .takes_value(true))
            .arg(Arg::with_name("format")
                .help("The format of the input")
                .value_name("json|ttl|tei")
                .short("f")
                .long("format")
                .takes_value(true))
            .arg(Arg::with_name("release")
                .help("The release level of the resource")
                .takes_value(true)
                .long("release")
                .value_name("PUBLIC|NONCOMMERCIAL|RESEARCH|PRIVATE"))
            .arg(Arg::with_name("genre")
                .help("The genre(s) of the dataset (comma separated)")
                .takes_value(true)
                .use_delimiter(true)
                .long("genre")
                .value_name("gen|lrn|ety|spe|his|ort|trm"))
            .arg(Arg::with_name("id")
                .help("The identifier of the dataset")
                .long("id")
                .takes_value(true))
            .arg(Arg::with_name("no_sql")
                .help("Do not use SQLite (all data is temporary and session only)")
                .long("no-sql"))                        
            .arg(Arg::with_name("config")
                .help("Configuration to help with mapping")
                .short("c")
                .long("config")
                .takes_value(true))
            .arg(Arg::with_name("db_path")
                .help("The path to use for the database (Default: eds.db)")
                .long("db-path")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("delete")
            .about("Delete a dictionary from the service")
            .arg(Arg::with_name("data")
                .help("Also load a single data file")
                .required(false)
                .index(1))
            .arg(Arg::with_name("db_path")
                .help("The path to use for the database (Default: eds.db)")
                .long("db-path")
                .takes_value(true)));
    let matches = app.clone().get_matches();
    if let Some(matches) = matches.subcommand_matches("load") {
        load_data(matches, &mut app);
    } else if let Some(matches) = matches.subcommand_matches("start") {
        let state = if matches.value_of("data").is_some() {
            load_data(matches, &mut app)
        } else {
            let path = matches.value_of("db_path").unwrap_or("eds.db");
            BackendImpl::DB(RusqliteState::new(path))
        };
        unsafe {
        ADDR.1 = u16::from_str(matches.value_of("port").unwrap_or("8000"))
            .unwrap_or_else(|_| show_help("Port is not an integer value", &mut app));
        }
        start_server(state);
    } else if let Some(matches) = matches.subcommand_matches("delete") {
        let path = matches.value_of("db_path").unwrap_or("eds.db");
        let name = matches.value_of("data").unwrap_or_else(|| 
            show_help("Please give a dictionary name to delete", &mut app));
        let db = RusqliteState::new(path);
        db.delete(name).unwrap();
    } else {
        show_help("Please give a command!", &mut app);
    }


}

fn fail(msg : &str) -> ! {
    eprintln!("{}",msg);
    std::process::exit(-1)
}

fn show_help(msg : &str, app : &mut App) -> ! {
    eprintln!("{}",msg);
    app.print_long_help().expect("Could not print help message!");
    std::process::exit(-1)
}

fn load_data(matches : &ArgMatches, app : &mut App) -> BackendImpl {
    let format = matches.value_of("data").unwrap_or("");
    let data : &str = matches.value_of("data").unwrap_or_else(|| show_help("The data paramter is required", app));
    let no_sql = matches.value_of("no_sql").is_some();
    let db_path = matches.value_of("db_path").unwrap_or("eds.db");
    let config = matches.value_of("config").and_then(|fname|  {
        serde_json::from_reader(File::open(fname)
            .unwrap_or_else(|e| fail(&format!("Could not open config file: {:?}", e)))
            ).unwrap_or_else(|e| fail(&format!("Could not parse config file: {:?}", e)))
    }).unwrap_or_else(|| Config::blank());
    let release = matches.value_of("release").and_then(|x| model::Release::from_str(x).ok()).unwrap_or_else(|| {
        if let Some(release) = config.default_release.clone() {
            release
        } else {
            eprintln!("Release is not specified or bad value, assuming PUBLIC");
            model::Release::PUBLIC
        }
    });
     
    if format == "json" || data.ends_with(".json") {
        let dictionaries : HashMap<String, DictJson> = serde_json::from_reader(
            File::open(data).
            unwrap_or_else(|e| fail(&format!("Could not open data file: {:?}", e)))).
            unwrap_or_else(|e| fail(&format!("Could not read dictionary file: {:?}", e)));
        let mut dict_map = HashMap::new();
        let mut entry_map = HashMap::new();
        for (id, dj) in dictionaries {
            dict_map.insert(id.clone(), dj.meta);
            entry_map.insert(id, dj.entries.into_iter().map(|x| EntryContent::Json(x)).collect());
        }
        if no_sql {
            BackendImpl::Mem(EDSState::new(release, dict_map, entry_map))
        } else {
            let db = RusqliteState::new(db_path);
            db.load(release,dict_map,entry_map).unwrap_or_else(|e| fail(&format!("Could not load database: {}", e.description())));
            BackendImpl::DB(db)
        }
    } else if format == "tei" || data.ends_with(".tei") || data.ends_with(".xml") {
        let mut genres = Vec::new();
        if let Some(gs) = matches.values_of("genre") {
            for g in gs {
                genres.push(model::Genre::from_str(g).
                    unwrap_or_else(|e| show_help(&e, app)));
            }
        };
        let id = matches.value_of("id").map(|x| x.to_owned()).unwrap_or_else(|| { 
            if let Some(id) = config.default_id.clone() {
                id
            } else {
                show_help("ID is required for TEI files",app)
            }
        });

        tei::parse(File::open(data)
            .unwrap_or_else(|e| fail(&format!("Could not open data file: {}", e.description()))), 
                &id, release, genres, &config, |r,d,e| {
                    if no_sql {
                        BackendImpl::Mem(EDSState::new(r,d,e))
                    } else {
                        let db = RusqliteState::new(db_path);
                        db.load(r,d,e).unwrap_or_else(|e| fail(&format!("Could not load database: {}", e.description())));
                        BackendImpl::DB(db)
                    }
                })
    } else if format == "ontolex" || data.ends_with(".rdf") || data.ends_with(".ttl") {
        let mut genres = Vec::new();
        if let Some(gs) = matches.values_of("genre") {
            for g in gs {
                genres.push(model::Genre::from_str(g).
                    unwrap_or_else(|e| show_help(&e, app)));
            }
        };

        ontolex::parse(File::open(data)
            .unwrap_or_else(|e| fail(&format!("Could not open data file: {}", e.description()))), 
                release, genres, &config, |r,d,e| {
                    if no_sql {
                        Ok(BackendImpl::Mem(EDSState::new(r,d,e)))
                    } else {
                        let db = RusqliteState::new(db_path);
                        db.load(r,d,e).unwrap_or_else(|e| fail(&format!("Could not load database: {}", e.description())));
                        Ok(BackendImpl::DB(db))
                    }
                }).unwrap_or_else(|e| fail(&format!("Could not read OntoLex file: {}", e.description())))
 
    } else {
        show_help(&format!("Unsupported format: {}", format),app);
    }
}

static mut ADDR : (&'static str, u16) = ("127.0.0.1",0);

fn start_server(state : BackendImpl) {
    unsafe {
    eprintln!("Starting server at {}:{}", ADDR.0, ADDR.1);
    gotham::start(&ADDR, router(state));
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictJson {
    meta : Dictionary,
    entries : Vec<JsonEntry>
}

#[derive(Clone,StateData)]
pub enum BackendImpl {
    Mem(EDSState),
    DB(RusqliteState)
}

impl model::Backend for BackendImpl {
    /// List the identifiers for all dictionaries
    fn dictionaries(&self) -> Result<Vec<String>,BackendError> {
        match self { 
            BackendImpl::Mem(s) => s.dictionaries(),
            BackendImpl::DB(s) => s.dictionaries()
        }
    }
    /// Obtain the metadata about a given dictionary
    fn about(&self, dictionary : &str) -> Result<Dictionary,BackendError> {
        match self { 
            BackendImpl::Mem(s) => s.about(dictionary),
            BackendImpl::DB(s) => s.about(dictionary)
        }
    }
    /// List all entries in a dictrionary
    fn list(&self, dictionary : &str, offset : Option<usize>, 
            limit : Option<usize>) -> Result<Vec<Entry>,BackendError> {
        match self { 
            BackendImpl::Mem(s) => s.list(dictionary, offset, limit),
            BackendImpl::DB(s) => s.list(dictionary, offset, limit)
        }
    }
    /// Search the dictionary by headword
    fn lookup(&self, dictionary : &str, headword : &str,
              offset : Option<usize>, limit : Option<usize>,
              part_of_speech : Option<PartOfSpeech>, inflected : bool) -> Result<Vec<Entry>,BackendError> {
        match self { 
            BackendImpl::Mem(s) => s.lookup(dictionary, headword, offset, limit, part_of_speech, inflected),
            BackendImpl::DB(s) => s.lookup(dictionary, headword, offset, limit, part_of_speech, inflected),
        }
    }
    /// Get the content as Json
    fn entry_json(&self, dictionary : &str, id : &str) -> Result<JsonEntry,BackendError> {
        match self { 
            BackendImpl::Mem(s) => s.entry_json(dictionary, id),
            BackendImpl::DB(s) => s.entry_json(dictionary, id),
        }
    }
    /// Get the content as OntoLex
    fn entry_ontolex(&self, dictionary : &str, id : &str) -> Result<String,BackendError> {
        match self { 
            BackendImpl::Mem(s) => s.entry_ontolex(dictionary, id),
            BackendImpl::DB(s) => s.entry_ontolex(dictionary, id)
        }
    }
    /// Get the content as TEI
    fn entry_tei(&self, dictionary : &str, id : &str) -> Result<String,BackendError> {
        match self { 
            BackendImpl::Mem(s) => s.entry_tei(dictionary, id),
            BackendImpl::DB(s) => s.entry_tei(dictionary, id)
        }
    }
}

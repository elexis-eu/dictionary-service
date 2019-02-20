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

mod rest;
mod state;

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

use clap::{App, Arg};

use std::fs::File;
use std::collections::HashMap;

use crate::state::{EDSState, Dictionary, EntryContent, PartOfSpeech};

fn router(state : EDSState) -> Router {
    let middleware = StateMiddleware::new(state);
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
    let matches = App::new("ELEXIS Dictionary Service")
                    .version("0.1")
                    .author("John P. McCrae <john@mccr.ae>")
                    .about("Server for hosting dictionaries so they may be accessed by the Dictionary Matrix")
                    .arg(Arg::with_name("data")
                         .help("The Json data to host")
                         .required(true)
                         .index(1))
                    .get_matches();

    let meta : &str = matches.value_of("data").expect("Meta paramter is required");
    let dictionaries : HashMap<String, DictJson> = serde_json::from_reader(File::open(meta)
                                               .expect("Could not open meta file"))
        .expect("Could not read dictionary file");
    let mut dict_map = HashMap::new();
    let mut entry_map = HashMap::new();
    for (id, dj) in dictionaries {
        dict_map.insert(id.clone(), dj.meta);
        entry_map.insert(id, dj.entries);
    }

    let addr = "127.0.0.1:8000";
    gotham::start(addr, router(EDSState::new(dict_map, entry_map)));
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictJson {
    meta : Dictionary,
    entries : Vec<EntryContent>
}

use http::{Response, StatusCode};
use gotham::state::State;
use hyper::Body;
use gotham::helpers::http::response::create_response;
use mime;
use crate::state::{EDSState, EntryContent};
use crate::{AboutParams, ListQueryParams, ListPathParams, LookupQueryParams, LookupPathParams, EntryPathParams};
use gotham::state::FromState;

#[derive(Serialize)]
struct DictionaryList {
    dictionaries : Vec<String>
}

/// Handle the "Get dictionaries" request
pub fn dictionaries(state : State) -> (State, Response<Body>) {
    let data = EDSState::borrow_from(&state);

    let list = DictionaryList {
        dictionaries : data.dictionaries.lock().unwrap().keys().map(|x| x.to_string()).collect()
    };

    let res = create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_vec(&list).expect("serialized dictionary list")
    );
    (state, res)
} 

/// Handle the "About the dictionary" request
pub fn about(state : State) -> (State, Response<Body>) {
    let data = EDSState::borrow_from(&state);
    let params = AboutParams::borrow_from(&state);

    let res = match data.dictionaries.lock().unwrap().get(&params.dictionary) {
        Some(dict) => {
            create_response(
                &state,
                StatusCode::OK,
                mime::APPLICATION_JSON,
                serde_json::to_vec(dict).expect("Cannot serialize metadata"))
        },
        None => {
            create_response(
                &state,
                StatusCode::NOT_FOUND,
                mime::TEXT_PLAIN,
                "Dictionary not found")
        }
    };
    (state, res)
}

/// Handle the "Get all lemmas" request
pub fn list(state : State) -> (State, Response<Body>) {
    let data = EDSState::borrow_from(&state);
    let params1 = ListPathParams::borrow_from(&state);
    let params2 = ListQueryParams::borrow_from(&state);

    let res = match data.entries.lock().unwrap().get(&params1.dictionary) {
        Some(emap) => {
            let entries : Vec<EntryContent> = match params2.offset {
                Some(offset) => {
                    match params2.limit {
                        Some(limit) => 
                            emap.values().flat_map(|x| x).skip(offset).take(limit).map(|x| x.clone()).collect(),
                        None =>
                            emap.values().flat_map(|x| x).skip(offset).map(|x| x.clone()).collect()
                    }
                },
                None =>
                    match params2.limit {
                        Some(limit) => 
                            emap.values().flat_map(|x| x).take(limit).map(|x| x.clone()).collect(),
                        None =>
                            emap.values().flat_map(|x| x).map(|x| x.clone()).collect()
                    }
            };
            create_response(
                &state,
                StatusCode::OK,
                mime::APPLICATION_JSON,
                serde_json::to_vec(&entries).expect("Cannot serialize entries"))
        }
        None => {
            create_response(
                &state,
                StatusCode::NOT_FOUND,
                mime::TEXT_PLAIN,
                "Dictionary not found")
        }
    };
    (state, res)
}

/// Handle the "Headword lookup" request
pub fn lookup(state : State) -> (State, Response<Body>) {
    let res = {
        let data = EDSState::borrow_from(&state);
        let params1 = LookupPathParams::borrow_from(&state);
        let params2 = LookupQueryParams::borrow_from(&state);

        let dict = data.entries.lock().unwrap();
        match dict.get(&params1.dictionary).and_then(|x| x.get(&params1.headword)) {
            Some(emap) => {
                let i1 = emap.iter()//.filter(|e| params2.language.is_none() || e.language == params2.language.unwrap())
                    .filter(|e| params2.part_of_speech.is_none() || Some(e.part_of_speech.convert()) == params2.part_of_speech);
                let entries : Vec<EntryContent> = match params2.offset {
                    Some(offset) => {
                        match params2.limit {
                            Some(limit) => 
                                i1.skip(offset).take(limit).map(|x| x.clone()).collect(),
                            None =>
                                i1.skip(offset).map(|x| x.clone()).collect()
                        }
                    },
                    None =>
                        match params2.limit {
                            Some(limit) => 
                                i1.take(limit).map(|x| x.clone()).collect(),
                            None =>
                                i1.map(|x| x.clone()).collect()
                        }
                };
                create_response(
                    &state,
                    StatusCode::OK,
                    mime::APPLICATION_JSON,
                    serde_json::to_vec(&entries).expect("Cannot serialize entries"))
            }
            None => {
                create_response(
                    &state,
                    StatusCode::NOT_FOUND,
                    mime::TEXT_PLAIN,
                    "Dictionary or entry not found")
            }
        }
    };
    (state, res)
}

/// Handle the "Entry as JSON" request
pub fn entry_json(state : State) -> (State, Response<Body>) {
    let res = {
        let data = EDSState::borrow_from(&state);
        let params1 = EntryPathParams::borrow_from(&state);
        match data.entries_by_id.lock().unwrap().get(&params1.dictionary).and_then(|x| x.get(&params1.id)) {
            Some(entry) => {
                create_response(
                    &state,
                    StatusCode::OK,
                    mime::APPLICATION_JSON,
                    serde_json::to_vec(entry).expect("Cannot serialize entry"))
            },
            None => {
                create_response(
                    &state,
                    StatusCode::NOT_FOUND,
                    mime::TEXT_PLAIN,
                    "Dictionary or entry not found")
            }
        }
    };
    (state, res)
}

/// Handle the "Entry as RDF" request
pub fn entry_ontolex(state : State) -> (State, Response<Body>) {
    let res = {
        create_response(
            &state,
            StatusCode::NOT_IMPLEMENTED,
            mime::TEXT_PLAIN,
            "TODO")
    };
    (state, res)
}

/// Handle the "Entry as TEI" request
pub fn entry_tei(state : State) -> (State, Response<Body>) {
    let res = {
        create_response(
            &state,
            StatusCode::NOT_IMPLEMENTED,
            mime::TEXT_PLAIN,
            "TODO")
    };
    (state, res)
}


     


use http::{Response, StatusCode};
use gotham::state::State;
use hyper::Body;
use gotham::helpers::http::response::create_response;
use mime;
use crate::model::{EDSState, Entry, EntryContent};
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

    let res = match data.entries_lemmas.lock().unwrap().get(&params1.dictionary) {
        Some(emap) => {
            let entries : Vec<Entry> = match params2.offset {
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

        let dict = data.entries_lemmas.lock().unwrap();
        let dict2 = data.entries_forms.lock().unwrap();
        match dict.get(&params1.dictionary).and_then(|x| x.get(&params1.headword)) {
            Some(emap) => {
                let i1 = emap.iter()
                    .filter(|e| params2.part_of_speech.is_none() || e.part_of_speech.contains(params2.part_of_speech.as_ref().unwrap()));
                let el = Vec::new();
                let i2 = (if params2.inflected == Some(true) {
                    match dict2.get(&params1.dictionary).and_then(|x| x.get(&params1.headword)) {
                        Some(emap2) => {
                            emap2.iter()
                        },
                        None => {
                            el.iter()
                        }
                    }
                } else {
                    el.iter()
                }).filter(|e| params2.part_of_speech.is_none() || e.part_of_speech.contains(params2.part_of_speech.as_ref().unwrap()));
                let entries : Vec<Entry> = match params2.offset {
                    Some(offset) => {
                        match params2.limit {
                            Some(limit) => 
                                i1.chain(i2).skip(offset).take(limit).map(|x| x.clone()).collect(),
                            None =>
                                i1.chain(i2).skip(offset).map(|x| x.clone()).collect()
                        }
                    },
                    None =>
                        match params2.limit {
                            Some(limit) => 
                                i1.chain(i2).take(limit).map(|x| x.clone()).collect(),
                            None =>
                                i1.chain(i2).map(|x| x.clone()).collect()
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
        match data.entries_id.lock().unwrap().get(&params1.dictionary).and_then(|x| x.get(&params1.id)) {
            Some(EntryContent::Json(entry)) => {
                create_response(
                    &state,
                    StatusCode::OK,
                    mime::APPLICATION_JSON,
                    serde_json::to_vec(entry).expect("Cannot serialize entry"))
            },
            Some(_) => {
                create_response(
                    &state,
                    StatusCode::NOT_FOUND,
                    mime::TEXT_PLAIN,
                    "Entry not available as Json")
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


     


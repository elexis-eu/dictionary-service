use http::{Response, StatusCode};
use gotham::state::State;
use hyper::Body;
use gotham::helpers::http::response::create_response;
use mime;
use crate::model::{EDSState, Backend};
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
        dictionaries : data.dictionaries()
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

    let res = match data.about(&params.dictionary) {
        Some(dict) => {
            create_response(
                &state,
                StatusCode::OK,
                mime::APPLICATION_JSON,
                serde_json::to_vec(&dict).expect("Cannot serialize metadata"))
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

    let res = match data.list(&params1.dictionary, params2.offset, params2.limit) {
        Some(entries) => {
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

        match data.lookup(&params1.dictionary, &params1.headword,
                          params2.offset, params2.limit,
                          params2.part_of_speech.clone(), params2.inflected.unwrap_or(false)) {
            Some(entries) => {
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
        match data.entry_json(&params1.dictionary, &params1.id) {
            Some(entry) => {
                create_response(
                    &state,
                    StatusCode::OK,
                    mime::APPLICATION_JSON,
                    serde_json::to_vec(&entry).expect("Cannot serialize entry"))
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
        let data = EDSState::borrow_from(&state);
        let params1 = EntryPathParams::borrow_from(&state);
        match data.entry_ontolex(&params1.dictionary, &params1.id) {
            Some(entry) => {
                create_response(
                    &state,
                    StatusCode::OK,
                    "text/turtle".parse().unwrap(),
                    serde_json::to_vec(&entry).expect("Cannot serialize entry"))
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

/// Handle the "Entry as TEI" request
pub fn entry_tei(state : State) -> (State, Response<Body>) {
    let res = {
        let data = EDSState::borrow_from(&state);
        let params1 = EntryPathParams::borrow_from(&state);
        match data.entry_tei(&params1.dictionary, &params1.id) {
            Some(entry) => {
                create_response(
                    &state,
                    StatusCode::OK,
                    mime::TEXT_XML,
                    entry)
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


     


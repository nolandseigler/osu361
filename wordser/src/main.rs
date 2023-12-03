// adapted from: https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs

use axum::{extract::Query, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use dotenv::dotenv;
use rust_bert::pipelines::summarization::SummarizationModel;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, net::SocketAddr};
use tokio::signal;

#[tokio::main]
async fn main() {
    dotenv().unwrap();
    // TODO: we would want to pass this into a handler but that proved to be a time suck due
    // to my lack of rust syntax experience. we want this to explode on start not when endpoint is called
    env::var("WEBSTER_THESAURUS_API_KEY").expect("WEBSTER_THESAURUS_API_KEY not in env");
    // build our application with a route
    let app = Router::new()
        .route("/api/v1/synonyms", get(handler_get_synonyms))
        .route("/api/v1/summary", get(handler_get_summary));

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("listening on {addr}");
    hyper::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

#[derive(Debug, Deserialize)]
struct GetSynonymsReq {
    word: String,
}

#[derive(Debug, Serialize)]
struct GetSynonymsResp {
    synonymns: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct ThesaurusPartialResp {
    meta: HashMap<String, serde_json::Value>,
}

async fn handler_get_synonyms(params: Query<GetSynonymsReq>) -> impl IntoResponse {
    println!("received text from wodrserweb service: {}", params.word);
    // TODO: I dont want to read from env everytime and I tried to use a closure to inject
    // the api-key during handler creation but it was too much of a pain
    // to try to do something that simple.
    let resp = reqwest::blocking::get(format!(
        "https://www.dictionaryapi.com/api/v3/references/thesaurus/json/{}?key={}",
        params.word,
        env::var("WEBSTER_THESAURUS_API_KEY").expect("WEBSTER_THESAURUS_API_KEY not in env"),
    ));

    if resp.is_err() {
        println!("resp: {:#?}", resp);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GetSynonymsResp {
                synonymns: Vec::new(),
            }),
        );
    }
    let resp = resp.unwrap();
    let partial: Result<Vec<ThesaurusPartialResp>, reqwest::Error> = resp.json();
    if partial.is_err() {
        println!("partial: {:#?}", partial);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GetSynonymsResp {
                synonymns: Vec::new(),
            }),
        );
    }
    let partial: Vec<ThesaurusPartialResp> = partial.unwrap();
    let meta_inner: &Vec<serde_json::Value> = match partial[0].meta.get("syns") {
        Some(m) => match m.as_array() {
            Some(s) => s,
            None => {
                println!("got None instead of syns");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GetSynonymsResp {
                        synonymns: Vec::new(),
                    }),
                );
            }
        },
        None => {
            println!("got None instead of syns");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetSynonymsResp {
                    synonymns: Vec::new(),
                }),
            );
        }
    };

    let mut syns: Vec<String> = Vec::new();
    for vect in meta_inner.iter() {
        let inner_vect = match vect.as_array() {
            Some(s) => s,
            None => {
                println!("got None instead of syns");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GetSynonymsResp {
                        synonymns: Vec::new(),
                    }),
                );
            }
        };
        for inner_v in inner_vect.iter() {
            let inner_str = match inner_v.as_str() {
                Some(s) => s,
                None => {
                    println!("got None instead of syns");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(GetSynonymsResp {
                            synonymns: Vec::new(),
                        }),
                    );
                }
            };
            syns.push(inner_str.to_string())
        }
    }
    println!("syns: {:?}", syns);
    let resp = GetSynonymsResp { synonymns: syns };

    (StatusCode::OK, Json(resp))
}

#[derive(Debug, Deserialize)]
struct GetSummaryReq {
    txt: String,
}

#[derive(Debug, Serialize)]
struct GetSummaryResp {
    summary: String,
}

async fn handler_get_summary(params: Query<GetSummaryReq>) -> impl IntoResponse {
    println!("txt to summarize: {:?}", params.txt);
    // TODO: This is probably stupid slow but idk how to use a closure or something to build this
    // during startup and pass it in......
    let summarization_model = match SummarizationModel::new(Default::default()) {
        Ok(m) => m,
        Err(_) => {
            println!("got None instead of syns");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GetSummaryResp {
                    summary: String::new(),
                }),
            );
        }
    };

    let input = ["In findings published Tuesday in Cornell University's arXiv by a team of scientists \
    from the University of Montreal and a separate report published Wednesday in Nature Astronomy by a team \
    from University College London (UCL), the presence of water vapour was confirmed in the atmosphere of K2-18b, \
    a planet circling a star in the constellation Leo. This is the first such discovery in a planet in its star's \
    habitable zone — not too hot and not too cold for liquid water to exist. The Montreal team, led by Björn Benneke, \
    used data from the NASA's Hubble telescope to assess changes in the light coming from K2-18b's star as the planet \
    passed between it and Earth. They found that certain wavelengths of light, which are usually absorbed by water, \
    weakened when the planet was in the way, indicating not only does K2-18b have an atmosphere, but the atmosphere \
    contains water in vapour form. The team from UCL then analyzed the Montreal team's data using their own software \
    and confirmed their conclusion. This was not the first time scientists have found signs of water on an exoplanet, \
    but previous discoveries were made on planets with high temperatures or other pronounced differences from Earth. \
    \"This is the first potentially habitable planet where the temperature is right and where we now know there is water,\" \
    said UCL astronomer Angelos Tsiaras. \"It's the best candidate for habitability right now.\" \"It's a good sign\", \
    said Ryan Cloutier of the Harvard–Smithsonian Center for Astrophysics, who was not one of either study's authors. \
    \"Overall,\" he continued, \"the presence of water in its atmosphere certainly improves the prospect of K2-18b being \
    a potentially habitable planet, but further observations will be required to say for sure. \"
    K2-18b was first identified in 2015 by the Kepler space telescope. It is about 110 light-years from Earth and larger \
    but less dense. Its star, a red dwarf, is cooler than the Sun, but the planet's orbit is much closer, such that a year \
    on K2-18b lasts 33 Earth days. According to The Guardian, astronomers were optimistic that NASA's James Webb space \
    telescope — scheduled for launch in 2021 — and the European Space Agency's 2028 ARIEL program, could reveal more \
    about exoplanets like K2-18b."];

    let output = summarization_model.summarize(&input);

    println!("summary: {:?}", output);

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(GetSummaryResp {
            summary: output[0].to_string(),
        }),
    )
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}

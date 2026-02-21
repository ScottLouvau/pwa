use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use once_cell::sync::Lazy;
use lib_wordle::wordle_tree::{WordleTree, tree_player};
use lib_wordle::{check, word::Word};

static ANSWERS: &str = std::include_str!("../../data/2315/answers.txt");
static VALID: &str = std::include_str!("../../data/2315/valid.txt");
static STRATEGY: &str = std::include_str!("../../data/v13.txt");
static INDEX: &str = std::include_str!("../index.html");

struct AppState {
    answers: Vec<Word>,
    valid: Vec<Word>,
}

impl AppState {
    fn new() -> AppState {
        let answers = Word::parse_lines(ANSWERS);
        let valid = Word::parse_lines(VALID);
        AppState { answers, valid }
    }
}

static APP_STATE: Lazy<AppState> = Lazy::new(|| { AppState::new() });

async fn assess(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if let Some(query) = req.uri().query() {
        let params = form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect::<HashMap<String, String>>();
        
        let guesses = params.get("g").as_ref().map(|g| g.as_str());

        match assess_inner(guesses, &APP_STATE.valid, &APP_STATE.answers) {
            Ok(result) => {
                return Ok(Response::builder()
                    .header("Content-Type", "text/plain; charset=utf-8")
                    .body(result.into())
                    .unwrap()
                );
            },
            Err(e) => {
                return Ok(Response::builder()
                    .status(400)
                    .body(e.into())
                    .unwrap()
                );
            }
        }
    }

    Ok(Response::builder()
        .status(400)
        .body("Must pass 'g' with Wordle guesses".into())
        .unwrap()
    )
}

fn assess_inner(guesses: Option<&str>, valid: &Vec<Word>, answers: &Vec<Word>) -> Result<String, String> {
    let simulate_game_count = 10000;

    let tree = WordleTree::parse(STRATEGY.lines())?;
    let player = tree_player::TreePlayer::new(&tree);

    return check::assess_and_simulate(guesses, valid, answers, simulate_game_count, player);
}

async fn index(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // Return embedded index.html
    Ok(Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(INDEX.into())
        .unwrap()
    )

    // Read and Return index.html (development)
    // match std::fs::read_to_string("./src/index.html") {
    //     Ok(index) => {
    //         Ok(Response::builder()
    //             .header("Content-Type", "text/html; charset=utf-8")
    //             .body(index.into())
    //             .unwrap()
    //         )
    //     },
    //     Err(e) => {
    //         eprintln!("Error reading index.html: {}", e);
    //         Ok(Response::builder()
    //             .status(500)
    //             .body("Internal Server Error".into())
    //             .unwrap()
    //         )
    //     }
    // }
}

async fn route(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match req.uri().path() {
        "/" => index(req).await,
        "/assess" => assess(req).await,
        _ => Ok(Response::builder()
            .status(404)
            .body("Not Found".into())
            .unwrap()
        )
    }
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Starting on {addr:?}...");


    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(route))
    });

    let server = Server::bind(&addr).serve(make_svc);
    //let server = server.with_graceful_shutdown(_shutdown_signal());

    if let Err(e) = server.await {
        eprintln!("Server Error: {}", e);
    }
}

async fn _shutdown_signal() {
    tokio::signal::ctrl_c().await.unwrap()
}
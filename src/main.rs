use axum::{extract::Extension, routing::get, Router, Server};
use std::{net::SocketAddr, sync::Arc};

/// Some shorthand utilities to interact with chains database
mod db_util {
    use markov::Chain;
    use sled::Db;

    /// Get chain from database by id
    pub fn get_chain(id: String, db: &Db) -> Chain<String> {
        db.get(id)
            .unwrap()
            .map(|v| rmp_serde::from_slice(&v.to_vec()).unwrap())
            .unwrap_or(markov::Chain::new())
    }

    /// Insert chain into database by id
    pub fn set_chain(id: String, chain: &Chain<String>, db: &Db) {
        db.insert(id, rmp_serde::to_vec(&chain).unwrap()).unwrap();
    }

    /// Delete chain from database by id
    pub fn delete_chain(id: String, db: &Db) {
        db.remove(id).unwrap();
    }
}

/// Http routes
mod routes {
    use crate::db_util;
    use axum::{extract::Path, http::StatusCode, Extension};
    use sled::Db;
    use std::sync::Arc;

    /// Returns generated text
    /// Fails if the chain is empty
    pub async fn generate(
        Extension(db): Extension<Arc<Db>>,
        Path(id): Path<String>,
    ) -> Result<String, StatusCode> {
        let chain = db_util::get_chain(id, &db);

        if chain.is_empty() {
            Err(StatusCode::NOT_ACCEPTABLE)
        } else {
            Ok(chain.generate_str())
        }
    }

    /// Feeds chain with input text
    pub async fn feed(
        text: String,
        Path(id): Path<String>,
        Extension(db): Extension<Arc<Db>>,
    ) -> StatusCode {
        let mut chain = db_util::get_chain(id.to_owned(), &db);
        chain.feed_str(&text);
        db_util::set_chain(id.to_owned(), &chain, &db);

        StatusCode::ACCEPTED
    }

    /// Clear chain
    pub async fn clear(Path(id): Path<String>, Extension(db): Extension<Arc<Db>>) -> StatusCode {
        db_util::delete_chain(id, &db);

        StatusCode::ACCEPTED
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("MCD_PORT")
        .expect("MCD_PORT is missing")
        .parse::<u16>()
        .unwrap();

    let db = Arc::new(
        sled::Config::new()
            .use_compression(true)
            .path("./data/db")
            .open()?,
    );

    let app = Router::new()
        .route(
            "/:id",
            get(routes::generate)
                .put(routes::feed)
                .delete(routes::clear),
        )
        .layer(Extension(db));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

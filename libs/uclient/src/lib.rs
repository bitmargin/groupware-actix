//! # Universal HTTP Client Interface for Rust
//!
//! `uclient` seeks to provide a unified interface for http client in rust.
//!
//![![Build Status](https://github.com/fMeow/uclient/workflows/CI%20%28Linux%29/badge.svg?branch=main)](https://github.com/fMeow/uclient/actions)
//![![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
//![![Crates.io](https://img.shields.io/crates/v/uclient.svg)](https://crates.io/crates/uclient)
//![![uclient](https://docs.rs/uclient/badge.svg)](https://docs.rs/uclient)
//!
//! Feature gates are used to conditionally enable specific http ecosystem.
//! Currently reqwest(both blocking and async) and surf(async only) are
//! supported out of the box.
//!
//! But it's possible to incorporate custom ecosystem. See
//! `examples/custom_client.rs`.
use http::{uri::Uri, HeaderMap, Request, Response};

pub use error::Error;
use std::io::{BufReader, Read};

mod error;
#[cfg(feature = "multipart")]
pub mod form;

#[cfg(any(
    all(feature = "async_reqwest", feature = "blocking_reqwest"),
    all(feature = "async_reqwest_rustls", feature = "blocking_reqwest"),
    all(feature = "async_reqwest", feature = "blocking_reqwest_rustls"),
    all(feature = "async_reqwest_rustls", feature = "blocking_reqwest_rustls"),
))]
compile_error!(r#"Enabling both async and blocking version of reqwest client is not allowed."#);

#[cfg(any(
    feature = "async_reqwest",
    feature = "blocking_reqwest",
    feature = "async_reqwest_rustls",
    feature = "blocking_reqwest_rustls"
))]
pub mod reqwest;
#[cfg(any(feature = "async_surf", feature = "async_surf_rustls"))]
pub mod surf;

#[maybe_async::maybe_async]
pub trait ClientExt: Sync + Clone {
    type Client;

    fn with_client(
        client: Self::Client,
        headers: impl Into<Option<HeaderMap>>,
    ) -> Result<Self, Error>;

    fn new(headers: impl Into<Option<HeaderMap>>) -> Result<Self, Error>;

    fn headers(&mut self) -> &mut HeaderMap;

    #[inline]
    async fn get<T>(&self, url: Uri, text: T) -> Result<Response<String>, Error>
    where
        T: Into<String> + Send,
    {
        self.request(Request::get(url.to_string()).body(text.into()).unwrap())
            .await
    }
    #[inline]
    async fn post<T>(&self, url: Uri, text: T) -> Result<Response<String>, Error>
    where
        T: Into<String> + Send,
    {
        self.request(Request::post(url.to_string()).body(text.into()).unwrap())
            .await
    }
    #[inline]
    async fn put<T>(&self, url: Uri, text: T) -> Result<Response<String>, Error>
    where
        T: Into<String> + Send,
    {
        self.request(Request::put(url.to_string()).body(text.into()).unwrap())
            .await
    }
    #[inline]
    async fn delete<T>(&self, url: Uri, text: T) -> Result<Response<String>, Error>
    where
        T: Into<String> + Send,
    {
        self.request(Request::delete(url.to_string()).body(text.into()).unwrap())
            .await
    }
    #[inline]
    async fn patch<T>(&self, url: Uri, text: T) -> Result<Response<String>, Error>
    where
        T: Into<String> + Send,
    {
        self.request(Request::patch(url.to_string()).body(text.into()).unwrap())
            .await
    }

    #[inline]
    async fn connect<T>(&self, url: Uri, text: T) -> Result<Response<String>, Error>
    where
        T: Into<String> + Send,
    {
        self.request(Request::connect(url.to_string()).body(text.into()).unwrap())
            .await
    }

    #[inline]
    async fn head<T>(&self, url: Uri, text: T) -> Result<Response<String>, Error>
    where
        T: Into<String> + Send,
    {
        self.request(Request::head(url.to_string()).body(text.into()).unwrap())
            .await
    }

    #[inline]
    async fn options<T>(&self, url: Uri, text: T) -> Result<Response<String>, Error>
    where
        T: Into<String> + Send,
    {
        self.request(Request::options(url.to_string()).body(text.into()).unwrap())
            .await
    }

    #[inline]
    async fn trace<T>(&self, url: Uri, text: T) -> Result<Response<String>, Error>
    where
        T: Into<String> + Send,
    {
        self.request(Request::trace(url.to_string()).body(text.into()).unwrap())
            .await
    }

    async fn request(&self, request: Request<String>) -> Result<Response<String>, Error> {
        self.request_bytes(request.map(|b| b.into_bytes())).await
    }

    async fn request_bytes(&self, request: Request<Vec<u8>>) -> Result<Response<String>, Error> {
        let req = request.map(|b| BufReader::new(std::io::Cursor::new(b)));
        self.request_reader(req).await
    }

    async fn request_reader<T>(&self, request: Request<T>) -> Result<Response<String>, Error>
    where
        T: Read + Send + Sync + 'static;
}

//! Reqwest HTTP client

#[cfg(any(feature = "blocking_reqwest", feature = "blocking_reqwest_rustls"))]
use ::reqwest::blocking::Client;

#[cfg(any(feature = "async_reqwest", feature = "async_reqwest_rustls"))]
use ::reqwest::Client;

use http::header::HeaderMap;

use crate::{ClientExt, Error};
use http::request::Parts;
use http::{HeaderValue, Request, Response};
use reqwest::redirect::Policy;
use std::io::Read;

#[derive(Debug, Clone)]
pub struct ReqwestClient {
    pub client: Client,
    headers: HeaderMap,
}

#[maybe_async::maybe_async]
impl ClientExt for ReqwestClient {
    type Client = Client;

    fn with_client(
        client: Self::Client,
        headers: impl Into<Option<HeaderMap>>,
    ) -> Result<Self, Error> {
        let headers = match headers.into() {
            Some(h) => h,
            None => HeaderMap::new(),
        };
        Ok(ReqwestClient { client, headers })
    }

    fn new(headers: impl Into<Option<HeaderMap>>) -> Result<Self, Error> {
        let client = Client::builder().gzip(true);
        let headers = match headers.into() {
            Some(h) => h,
            None => HeaderMap::new(),
        };

        client
            .redirect(Policy::none())
            .build()
            .map(|c| ReqwestClient { client: c, headers })
            .map_err(|e| Error::HttpClient(format!("{:?}", e)))
    }

    fn headers(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.headers
    }

    async fn request_reader<T>(&self, mut request: Request<T>) -> Result<Response<String>, Error>
    where
        T: Read + Send + Sync + 'static,
    {
        let headers = request.headers_mut();
        for (header, value) in self.headers.iter() {
            if !headers.contains_key(header) {
                headers.insert(header, value.clone());
            }
        }

        let req = get_req(request);
        let resp = self
            .client
            .execute(req)
            .await
            .map_err(|e| Error::HttpClient(format!("{:?}", e)))?;

        let status_code = resp.status();
        let headers = resp.headers().clone();
        let version = resp.version();
        let content = resp
            .text()
            .await
            .map_err(|e| Error::HttpClient(format!("{:?}", e)))?;
        let mut build = http::Response::builder();

        for header in headers.iter() {
            build = build.header(header.0, header.1);
        }

        build
            .status(status_code)
            .version(version)
            .body(content)
            .map_err(|e| Error::HttpClient(format!("{:?}", e)))
    }
}

#[maybe_async::async_impl]
fn get_req<T>(req: Request<T>) -> reqwest::Request
where
    T: Read + Send + Sync + 'static,
{
    use futures::StreamExt;
    use reqwest::Body;

    let (parts, body) = req.into_parts();
    let Parts {
        method,
        uri,
        headers,
        ..
    } = parts;

    let mut request = reqwest::Request::new(method, uri.to_string().parse().unwrap());

    let mut prev_name = None;
    for (key, value) in headers {
        match key {
            Some(key) => {
                request.headers_mut().insert(key.clone(), value);
                prev_name = Some(key);
            }
            None => match prev_name {
                Some(ref key) => {
                    request.headers_mut().append(key.clone(), value);
                }
                None => unreachable!("HeaderMap::into_iter yielded None first"),
            },
        }
    }
    let body_bytes = body.bytes();
    let stream = futures::stream::iter(body_bytes).chunks(2048).map(|x| {
        let len = x.len();
        let out = x.into_iter().filter_map(|b| b.ok()).collect::<Vec<_>>();
        if out.len() == len {
            Ok(out)
        } else {
            Err(crate::Error::PayloadError)
        }
    });
    request.body_mut().replace(Body::wrap_stream(stream));

    request
}

#[maybe_async::sync_impl]
fn get_req<T>(req: Request<T>) -> reqwest::blocking::Request
where
    T: Read + Send + Sync + 'static,
{
    use reqwest::blocking::Body;

    let (parts, body) = req.into_parts();
    let Parts {
        method,
        uri,
        headers,
        ..
    } = parts;
    let mut request = reqwest::blocking::Request::new(method, uri.to_string().parse().unwrap());

    let mut prev_name = None;
    for (key, value) in headers {
        match key {
            Some(key) => {
                request.headers_mut().insert(key.clone(), value);
                prev_name = Some(key);
            }
            None => match prev_name {
                Some(ref key) => {
                    request.headers_mut().append(key.clone(), value);
                }
                None => unreachable!("HeaderMap::into_iter yielded None first"),
            },
        }
    }
    request.body_mut().replace(Body::new(body));

    request
}

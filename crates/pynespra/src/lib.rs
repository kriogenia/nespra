//! Python bindings of the `nespra` client.
//!
//! This extension is only the transport: it sends the already serialized request and
//! returns the status and the body undecoded, using `Vespa::search_raw`. Serialization,
//! decoding, error classification and retries are done in the Python package, which
//! keeps this layer as fast and thin as possible.

use std::time::Duration;

use nespra::{
    Format, HttpVersion, Vespa,
    error::SearchError,
    reqwest::header::{HeaderName, HeaderValue},
    search::SearchOptions,
};
use pyo3::{create_exception, exceptions::PyException, prelude::*, types::PyBytes};

create_exception!(
    _core,
    NespraError,
    PyException,
    "Base class of the nespra errors."
);
create_exception!(
    _core,
    BuildError,
    NespraError,
    "The client could not be built."
);
create_exception!(
    _core,
    RequestError,
    NespraError,
    "The request could not be sent or the response could not be read."
);

fn build_error(message: impl ToString) -> PyErr {
    BuildError::new_err(message.to_string())
}

/// Maps a failed request, tagging it with whether repeating it can succeed.
fn request_error(error: SearchError) -> PyErr {
    let retryable = error.is_retryable();
    let err = RequestError::new_err(error.to_string());
    Python::attach(|py| {
        let _ = err.value(py).setattr("retryable", retryable);
    });
    err
}

fn seconds(name: &str, value: f64) -> PyResult<Duration> {
    Duration::try_from_secs_f64(value)
        .map_err(|_| build_error(format!("{name} must be a non-negative number of seconds")))
}

/// Sends raw search requests to Vespa.
#[pyclass(module = "nespra._core", frozen)]
struct Transport {
    vespa: Vespa,
}

#[pymethods]
impl Transport {
    #[new]
    #[pyo3(signature = (
        url,
        *,
        bearer_token = None,
        client_cert = None,
        client_key = None,
        headers = None,
        user_agent = None,
        timeout = None,
        connect_timeout = None,
        http_version = "HTTP/2",
        format = "cbor",
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        url: String,
        bearer_token: Option<String>,
        client_cert: Option<Vec<u8>>,
        client_key: Option<Vec<u8>>,
        headers: Option<Vec<(String, String)>>,
        user_agent: Option<String>,
        timeout: Option<f64>,
        connect_timeout: Option<f64>,
        http_version: &str,
        format: &str,
    ) -> PyResult<Self> {
        let mut builder = Vespa::builder(url)
            .format(format.parse::<Format>().map_err(build_error)?)
            .http_version(http_version.parse::<HttpVersion>().map_err(build_error)?);

        if let Some(token) = bearer_token {
            builder = builder.bearer_token(&token);
        }

        match (client_cert, client_key) {
            (Some(cert), Some(key)) => builder = builder.client_pem(&cert, &key),
            (None, None) => {}
            _ => return Err(build_error("client_cert and client_key go together")),
        }

        for (name, value) in headers.unwrap_or_default() {
            let name = HeaderName::try_from(name.as_str()).map_err(build_error)?;
            let value = HeaderValue::from_str(&value).map_err(build_error)?;
            builder = builder.header(name, value);
        }

        if let Some(user_agent) = user_agent {
            builder = builder.user_agent(HeaderValue::from_str(&user_agent).map_err(build_error)?);
        }

        if let Some(timeout) = timeout {
            builder = builder.timeout(seconds("timeout", timeout)?);
        }

        if let Some(timeout) = connect_timeout {
            builder = builder.connect_timeout(seconds("connect_timeout", timeout)?);
        }

        let vespa = builder.build().map_err(build_error)?;
        Ok(Self { vespa })
    }

    /// Sends the request and returns `(status, body)`, blocking the thread but not the GIL.
    #[pyo3(signature = (body, timeout = None))]
    fn request<'py>(
        &self,
        py: Python<'py>,
        body: &[u8],
        timeout: Option<f64>,
    ) -> PyResult<(u16, Bound<'py, PyBytes>)> {
        let options = options(timeout)?;
        let response = py
            .detach(|| {
                pyo3_async_runtimes::tokio::get_runtime()
                    .block_on(self.vespa.search_raw_with(body, &options))
            })
            .map_err(request_error)?;
        Ok((response.status.as_u16(), PyBytes::new(py, &response.body)))
    }

    /// Same as `request` but returns an awaitable.
    #[pyo3(signature = (body, timeout = None))]
    fn request_async<'py>(
        &self,
        py: Python<'py>,
        body: Vec<u8>,
        timeout: Option<f64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let options = options(timeout)?;
        let vespa = self.vespa.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let response = vespa
                .search_raw_with(&body, &options)
                .await
                .map_err(request_error)?;
            Ok((response.status.as_u16(), response.body.to_vec()))
        })
    }
}

fn options(timeout: Option<f64>) -> PyResult<SearchOptions> {
    let mut options = SearchOptions::new();
    if let Some(timeout) = timeout {
        options = options.timeout(seconds("timeout", timeout)?);
    }
    Ok(options)
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Transport>()?;
    m.add("NespraError", m.py().get_type::<NespraError>())?;
    m.add("BuildError", m.py().get_type::<BuildError>())?;
    m.add("RequestError", m.py().get_type::<RequestError>())?;
    Ok(())
}

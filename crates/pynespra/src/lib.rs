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
    }
}

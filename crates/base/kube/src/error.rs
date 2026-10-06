use std::error::Error as _;

/// What a call to a cluster could not do.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no kubeconfig names the context {0}")]
    NoContext(String),
    #[error("the kubeconfig could not be read: {0}")]
    Kubeconfig(String),
    /// The credentials were refused or could not be made: a sign-in fixes it.
    #[error("{context} refused the sign-in: {detail}")]
    Refused { context: String, detail: String },
    #[error("{context} did not answer: {detail}")]
    Unreachable { context: String, detail: String },
    #[error("{context} answered {status}: {message}")]
    Api {
        context: String,
        status: u16,
        message: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// A kube-rs error, sorted for the context it came from.
    pub(crate) fn of(context: &str, error: kube::Error) -> Self {
        let context = context.to_string();
        match error {
            kube::Error::Api(status) if status.code == 401 => Error::Refused {
                context,
                detail: status.message,
            },
            kube::Error::Api(status) => Error::Api {
                context,
                status: status.code,
                message: status.message,
            },
            kube::Error::Auth(e) => Error::Refused {
                context,
                detail: e.to_string(),
            },
            kube::Error::InferKubeconfig(e) => Error::Kubeconfig(e.to_string()),
            other if signs_in(&other) => Error::Refused {
                context,
                detail: other.to_string(),
            },
            other => Error::Unreachable {
                context,
                detail: other.to_string(),
            },
        }
    }
}

fn signs_in(error: &kube::Error) -> bool {
    let mut source = error.source();
    while let Some(one) = source {
        if one.is::<kube::client::AuthError>() {
            return true;
        }
        source = one.source();
    }
    false
}

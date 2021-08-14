use {
    std::{
        future::Future,
        pin::Pin,
        sync::Arc,
        task::{Context, Poll},
    },
    tokio::sync::RwLock,
};

pub struct Example;

pub enum Request {
    Name,
    Echo { string: String },
    Reverse { string: String },
}

impl Example {
    pub fn name() -> String {
        "Example".to_owned()
    }

    pub fn echo(&self, string: String) -> String {
        string
    }

    pub async fn reverse(&mut self, string: String) -> Result<String, EmptyString> {
        if !string.is_empty() {
            Ok(string.chars().rev().collect())
        } else {
            Err(EmptyString)
        }
    }
}

pub struct Service(Arc<RwLock<Example>>);

impl Service {
    pub async fn name(&mut self) -> String {
        use tower::{Service as _, ServiceExt as _};

        let service = self
            .ready()
            .await
            .expect("Generated service is always ready");

        service
            .call(Request::Name)
            .await
            .expect("Result data never fails")
    }

    pub async fn echo(&mut self, string: String) -> String {
        use tower::{Service as _, ServiceExt as _};

        let service = self
            .ready()
            .await
            .expect("Generated service is always ready");

        service
            .call(Request::Echo { string })
            .await
            .expect("Result data never fails")
    }

    pub async fn reverse(&mut self, string: String) -> Result<String, EmptyString> {
        use tower::{Service as _, ServiceExt as _};

        let service = self
            .ready()
            .await
            .expect("Generated service is always ready");

        service.call(Request::Reverse { string }).await
    }
}

impl tower::Service<Request> for Service {
    type Response = String;
    type Error = EmptyString;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request) -> Self::Future {
        use futures::FutureExt as _;

        let inner = self.0.clone();

        async move {
            match request {
                Request::Name => Ok(Example::name()),
                Request::Echo { string } => Ok(inner.read().await.echo(string)),
                Request::Reverse { string } => inner.write().await.reverse(string).await,
            }
        }
        .boxed()
    }
}

#[derive(Debug)]
pub struct EmptyString;

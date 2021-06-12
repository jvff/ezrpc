# EZRPC

A procedural macro crate to reduce boiler plate code when writing RPC interfaces. The aim is to
generate as much code as possible, and let development focus on the actual implementation. This is
currently a proof-of-concept, and currently only supports `tower`.

## Example

There is an example project inside the `example` directory. The summary is that this crate allows
one to write the following:

```rust
pub struct Example;

#[ezrpc::tower]
impl Example {
    pub async fn echo(string: String) -> Result<String, EmptyString> {
        if !string.is_empty() {
            Ok(string)
        } else {
            Err(EmptyString)
        }
    }

    pub async fn reverse(string: String) -> Result<String, EmptyString> {
        if !string.is_empty() {
            Ok(string.chars().rev().collect())
        } else {
            Err(EmptyString)
        }
    }
}

pub struct EmptyString;
```

and get the following code:

```rust
pub struct Example;

pub enum Request {
    Echo { string: String },
    Reverse { string: String },
}

impl Example {
    pub async fn echo(string: String) -> Result<String, EmptyString> {
        if !string.is_empty() {
            Ok(string)
        } else {
            Err(EmptyString)
        }
    }
    pub async fn reverse(string: String) -> Result<String, EmptyString> {
        if !string.is_empty() {
            Ok(string.chars().rev().collect())
        } else {
            Err(EmptyString)
        }
    }
}

pub struct Service;

impl tower::Service<Request> for Service {
    type Response = String;
    type Error = EmptyString;
    type Future =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(
        &mut self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request) -> Self::Future {
        match request {
            Request::Echo { string } => futures::FutureExt::boxed(Example::echo(string)),
            Request::Reverse { string } => futures::FutureExt::boxed(Example::reverse(string)),
        }
    }
}

impl Service {
    pub async fn echo(&mut self, string: String) -> Result<String, EmptyString> {
        use tower::{Service as _, ServiceExt as _};

        self.ready().await?.call(Request::Echo { string }).await
    }

    pub async fn reverse(&mut self, string: String) -> Result<String, EmptyString> {
        use tower::{Service as _, ServiceExt as _};

        self.ready().await?.call(Request::Reverse { string }).await
    }
}

pub struct EmptyString;
```
